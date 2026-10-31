'use client';

import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { CircleCheck, CircleX, X, Plus, Trash2 } from 'lucide-react';
import { DynamicCodeBlock } from 'fumadocs-ui/components/dynamic-codeblock';
import { buttonVariants } from 'fumadocs-ui/components/ui/button';
import { twMerge } from 'tailwind-merge';

interface VhostConfig {
  hostname: string;
  root: string;
  spa: string;
  spaIgnore: string;
  customIndex: string;
  custom404: string;
  corsOrigin: string;
}

interface ServerConfig {
  listen: string;
  behindProxy: string;
  proxyDepth: number;
  maxConnections: number;
  connectionTimeout: number;
  maxFileSize: number;
  maxFileSizeUnit: string;
  logging: string;
  logPath: string;
  logTimezone: string;
}

interface ValidationResult {
  valid: boolean;
  errors: string[];
}

const FILE_SIZE_MULTIPLIERS: Record<string, number> = {
  MB: 1024 * 1024,
  GB: 1024 * 1024 * 1024,
};

function fileSizeToBytes(value: number, unit: string): number {
  return value * (FILE_SIZE_MULTIPLIERS[unit] || 1);
}

// JSON string escaping is valid TOML basic-string escaping
const str = (s: string) => JSON.stringify(s);

function generateToml(server: ServerConfig, vhosts: VhostConfig[]): string {
  const lines: string[] = [];
  const maxBytes = fileSizeToBytes(server.maxFileSize, server.maxFileSizeUnit);

  lines.push('[server]');
  lines.push(`listen = ${str(server.listen)}`);
  if (server.behindProxy === 'enabled') {
    lines.push('behind_proxy = true');
    lines.push(`trusted_proxy_depth = ${server.proxyDepth}`);
  }
  if (server.maxConnections !== 1024)
    lines.push(`max_connections = ${server.maxConnections}`);
  if (server.connectionTimeout !== 30)
    lines.push(`connection_timeout = ${server.connectionTimeout}`);
  if (maxBytes !== 52428800)
    lines.push(`max_file_size = ${maxBytes}`);

  if (server.logging === 'enabled') {
    lines.push('');
    lines.push('[logging]');
    lines.push('enabled = true');
    lines.push(`path = ${str(server.logPath)}`);
    if (server.logTimezone !== 'UTC')
      lines.push(`timezone = ${str(server.logTimezone)}`);
  }

  for (const vhost of vhosts) {
    lines.push('');
    lines.push('[[vhost]]');
    lines.push(`hostnames = [${str(vhost.hostname)}]`);
    lines.push(`root = ${str(vhost.root)}`);

    if (vhost.spa === 'enabled') lines.push('spa = true');
    if (vhost.spa === 'enabled' && vhost.spaIgnore.trim()) {
      const parts = vhost.spaIgnore
        .split(',')
        .map((s) => s.trim())
        .filter(Boolean)
        .map(str)
        .join(', ');
      lines.push(`spa_ignore = [${parts}]`);
    }
    if (vhost.customIndex && vhost.customIndex !== 'index.html')
      lines.push(`index = ${str(vhost.customIndex)}`);
    if (vhost.custom404) lines.push(`custom_404 = ${str(vhost.custom404)}`);

    if (vhost.corsOrigin.trim()) {
      lines.push('');
      lines.push('[vhost.headers]');
      lines.push(`Access-Control-Allow-Origin = ${str(vhost.corsOrigin)}`);
    }
  }

  return lines.join('\n');
}

function validate(server: ServerConfig, vhosts: VhostConfig[]): ValidationResult {
  const errors: string[] = [];
  const maxBytes = fileSizeToBytes(server.maxFileSize, server.maxFileSizeUnit);

  if (!server.listen.trim()) errors.push('Listen address is required');
  if (server.maxConnections < 1 || server.maxConnections > 4096)
    errors.push('Max connections must be 1–4096');
  if (server.connectionTimeout < 1 || server.connectionTimeout > 3600)
    errors.push('Connection timeout must be 1–3600');
  if (maxBytes < 1 || maxBytes > 1073741824)
    errors.push('Max file size must be between 1 byte and 1 GB');
  if (server.behindProxy === 'enabled' && (server.proxyDepth < 1 || server.proxyDepth > 16))
    errors.push('Proxy depth must be 1–16');
  if (server.logging === 'enabled' && !server.logPath.trim())
    errors.push('Log path is required when logging is enabled');

  if (vhosts.length === 0) errors.push('At least one vhost is required');

  for (let i = 0; i < vhosts.length; i++) {
    const v = vhosts[i];
    if (!v.hostname.trim()) errors.push(`Vhost ${i + 1}: hostname is required`);
    if (!v.root.trim()) errors.push(`Vhost ${i + 1}: root is required`);
    if (v.root === '/') errors.push(`Vhost ${i + 1}: root cannot be /`);
  }

  const hostnames = vhosts.map((v) => v.hostname);
  if (new Set(hostnames).size !== hostnames.length)
    errors.push('Duplicate hostnames detected');

  return { valid: errors.length === 0, errors };
}

const inputCls =
  'rounded-md border border-fd-border bg-fd-secondary px-3 py-1.5 text-sm text-fd-foreground placeholder:text-fd-muted-foreground focus:outline-none focus:ring-2 focus:ring-fd-ring w-full';
const monoInputCls = `${inputCls} font-mono`;
const selectCls = `${inputCls} pr-9`;
const selectStyle: React.CSSProperties = {
  appearance: 'none',
  backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%2371717a' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E")`,
  backgroundRepeat: 'no-repeat',
  backgroundPosition: 'right 0.625rem center',
  backgroundSize: '16px 16px',
};
const labelCls = 'text-xs font-medium text-fd-muted-foreground';

export function RustinxConfigurator() {
  const [server, setServer] = useState<ServerConfig>({
    listen: '0.0.0.0:9090',
    behindProxy: 'enabled',
    proxyDepth: 1,
    maxConnections: 1024,
    connectionTimeout: 30,
    maxFileSize: 50,
    maxFileSizeUnit: 'MB',
    logging: 'disabled',
    logPath: '/var/log/rustinx/access.log',
    logTimezone: 'UTC',
  });

  const [vhosts, setVhosts] = useState<VhostConfig[]>([
    {
      hostname: 'rustinx.shadowarcanist.com',
      root: '/static',
      spa: 'enabled',
      spaIgnore: '/assets',
      customIndex: 'index.html',
      custom404: '',
      corsOrigin: '',
    },
  ]);

  const [toast, setToast] = useState<ValidationResult | null>(null);
  const [toastVisible, setToastVisible] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(null);

  const showToast = useCallback((result: ValidationResult) => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setToast(result);
    setToastVisible(true);
    timerRef.current = setTimeout(() => {
      setToastVisible(false);
      setTimeout(() => setToast(null), 300);
    }, 5000);
  }, []);

  const dismissToast = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setToastVisible(false);
    setTimeout(() => setToast(null), 300);
  }, []);

  useEffect(() => () => { if (timerRef.current) clearTimeout(timerRef.current); }, []);

  const updateServer = <K extends keyof ServerConfig>(field: K, value: ServerConfig[K]) =>
    setServer((prev) => ({ ...prev, [field]: value }));

  const updateVhost = (idx: number, field: keyof VhostConfig, value: string) =>
    setVhosts((prev) => prev.map((v, i) => (i === idx ? { ...v, [field]: value } : v)));

  const addVhost = () =>
    setVhosts((prev) => [...prev, { hostname: '', root: '/static', spa: 'disabled', spaIgnore: '', customIndex: 'index.html', custom404: '', corsOrigin: '' }]);

  const removeVhost = (idx: number) =>
    setVhosts((prev) => prev.filter((_, i) => i !== idx));

  const output = useMemo(() => generateToml(server, vhosts), [server, vhosts]);

  return (
    <div className="flex flex-col gap-6 not-prose">
      {/* Server */}
      <div className="rounded-xl border border-fd-border bg-fd-card p-5">
        <h3 className="text-lg font-semibold text-fd-card-foreground m-0 mb-4">Server</h3>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div className="flex flex-col gap-1.5">
            <label className={labelCls}>Listen Address</label>
            <input type="text" value={server.listen} onChange={(e) => updateServer('listen', e.target.value)} placeholder="0.0.0.0:9090" className={monoInputCls} />
          </div>
          <div className="flex flex-col gap-1.5">
            <label className={labelCls}>Max Connections</label>
            <input type="number" value={server.maxConnections} onChange={(e) => updateServer('maxConnections', parseInt(e.target.value) || 1024)} className={inputCls} />
          </div>
          <div className="flex flex-col gap-1.5">
            <label className={labelCls}>Connection Timeout (s)</label>
            <input type="number" value={server.connectionTimeout} onChange={(e) => updateServer('connectionTimeout', parseInt(e.target.value) || 30)} className={inputCls} />
          </div>
          <div className="flex flex-col gap-1.5">
            <label className={labelCls}>Max File Size</label>
            <div className="flex gap-2">
              <input type="number" value={server.maxFileSize} onChange={(e) => updateServer('maxFileSize', parseInt(e.target.value) || 50)} className={`${inputCls} flex-1`} />
              <select value={server.maxFileSizeUnit} onChange={(e) => updateServer('maxFileSizeUnit', e.target.value)} className={selectCls} style={{ ...selectStyle, width: '5rem' }}>
                <option value="MB">MB</option>
                <option value="GB">GB</option>
              </select>
            </div>
          </div>
          <div className="flex flex-col gap-1.5">
            <label className={labelCls}>Behind Proxy</label>
            <select value={server.behindProxy} onChange={(e) => updateServer('behindProxy', e.target.value)} className={selectCls} style={selectStyle}>
              <option value="enabled">Enabled</option>
              <option value="disabled">Disabled</option>
            </select>
          </div>
          <div className="flex flex-col gap-1.5">
            <label className={labelCls}>Access Logging</label>
            <select value={server.logging} onChange={(e) => updateServer('logging', e.target.value)} className={selectCls} style={selectStyle}>
              <option value="disabled">Disabled</option>
              <option value="enabled">Enabled</option>
            </select>
          </div>
        </div>

        {server.behindProxy === 'enabled' && (
          <div className="mt-3 grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div className="flex flex-col gap-1.5">
              <label className={labelCls}>Trusted Proxy Depth</label>
              <input type="number" value={server.proxyDepth} onChange={(e) => updateServer('proxyDepth', parseInt(e.target.value) || 1)} min={1} max={16} className={inputCls} />
            </div>
          </div>
        )}

        {server.logging === 'enabled' && (
          <div className="mt-3 grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div className="flex flex-col gap-1.5">
              <label className={labelCls}>Log Path</label>
              <input type="text" value={server.logPath} onChange={(e) => updateServer('logPath', e.target.value)} className={monoInputCls} />
            </div>
            <div className="flex flex-col gap-1.5">
              <label className={labelCls}>Timezone</label>
              <select value={server.logTimezone} onChange={(e) => updateServer('logTimezone', e.target.value)} className={selectCls} style={selectStyle}>
                <option value="UTC">UTC (+00:00)</option>
                <option value="+09:00">JST (+09:00)</option>
                <option value="+01:00">CET (+01:00)</option>
                <option value="+02:00">EET (+02:00)</option>
                <option value="+08:00">CST (+08:00)</option>
                <option value="-05:00">EST (-05:00)</option>
                <option value="-06:00">CST (-06:00)</option>
                <option value="-07:00">MST (-07:00)</option>
                <option value="-08:00">PST (-08:00)</option>
              </select>
            </div>
          </div>
        )}
      </div>

      {/* Virtual Hosts */}
      {vhosts.map((vhost, vi) => (
        <div key={vi} className="rounded-xl border border-fd-border bg-fd-card p-5">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-lg font-semibold text-fd-card-foreground m-0">
              Virtual Host {vhosts.length > 1 ? `#${vi + 1}` : ''}
            </h3>
            {vhosts.length > 1 && (
              <button type="button" onClick={() => removeVhost(vi)} className="text-red-500 hover:text-red-400 p-1">
                <Trash2 size={16} />
              </button>
            )}
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div className="flex flex-col gap-1.5">
              <label className={labelCls}>Hostname <span className="text-fd-muted-foreground/60">(_ = catch-all)</span></label>
              <input type="text" value={vhost.hostname} onChange={(e) => updateVhost(vi, 'hostname', e.target.value)} placeholder="_" className={inputCls} />
            </div>
            <div className="flex flex-col gap-1.5">
              <label className={labelCls}>Root Directory</label>
              <input type="text" value={vhost.root} onChange={(e) => updateVhost(vi, 'root', e.target.value)} placeholder="/static" className={monoInputCls} />
            </div>
            <div className="flex flex-col gap-1.5">
              <label className={labelCls}>Index File</label>
              <input type="text" value={vhost.customIndex} onChange={(e) => updateVhost(vi, 'customIndex', e.target.value)} placeholder="index.html" className={inputCls} />
            </div>
            <div className="flex flex-col gap-1.5">
              <label className={labelCls}>Custom 404 Page</label>
              <input type="text" value={vhost.custom404} onChange={(e) => updateVhost(vi, 'custom404', e.target.value)} placeholder="404.html" className={inputCls} />
            </div>
            <div className="flex flex-col gap-1.5">
              <label className={labelCls}>SPA Mode</label>
              <select value={vhost.spa} onChange={(e) => updateVhost(vi, 'spa', e.target.value)} className={selectCls} style={selectStyle}>
                <option value="enabled">Enabled</option>
                <option value="disabled">Disabled</option>
              </select>
            </div>
            <div className="flex flex-col gap-1.5">
              <label className={labelCls}>CORS Allow Origin</label>
              <input type="text" value={vhost.corsOrigin} onChange={(e) => updateVhost(vi, 'corsOrigin', e.target.value)} placeholder="https://mysite.com" className={monoInputCls} />
            </div>
          </div>

          {vhost.spa === 'enabled' && (
            <div className="mt-3 grid grid-cols-1 gap-3">
              <div className="flex flex-col gap-1.5">
                <label className={labelCls}>SPA Ignore Prefixes <span className="text-fd-muted-foreground/60">(comma separated)</span></label>
                <input type="text" value={vhost.spaIgnore} onChange={(e) => updateVhost(vi, 'spaIgnore', e.target.value)} placeholder="/assets, /api" className={monoInputCls} />
              </div>
            </div>
          )}
        </div>
      ))}

      <button type="button" onClick={addVhost} className={twMerge(buttonVariants({ variant: 'outline' }), 'w-full')}>
        <Plus size={16} className="mr-1" /> Add Virtual Host
      </button>

      {/* Output */}
      <div className="rounded-xl border border-fd-border overflow-hidden">
        <div className="flex items-center border-b border-fd-border bg-fd-muted/50 px-4 py-2">
          <span className="text-sm font-medium text-fd-card-foreground">rustinx.toml</span>
          <div className="ml-auto">
            <button
              type="button"
              onClick={() => showToast(validate(server, vhosts))}
              className={twMerge(buttonVariants({ color: 'secondary', size: 'sm' }), 'gap-1.5 [&_svg]:size-3.5')}
            >
              Validate
            </button>
          </div>
        </div>
        <DynamicCodeBlock lang="toml" code={output} />
      </div>

      {/* Toast */}
      {toast && (
        <div className={`fixed bottom-6 right-6 z-50 max-w-sm transition-all duration-300 ${toastVisible ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-2'}`}>
          <div
            className={`flex gap-2 rounded-xl border bg-fd-card p-3 ps-1 text-sm text-fd-card-foreground shadow-md ${toast.valid ? 'items-center' : 'items-start'}`}
            style={{ '--callout-color': toast.valid ? 'var(--color-fd-success, #22c55e)' : 'var(--color-fd-error, #ef4444)' } as React.CSSProperties}
          >
            <div role="none" className="w-0.5 rounded-sm" style={{ backgroundColor: 'color-mix(in srgb, var(--callout-color) 50%, transparent)' }} />
            {toast.valid ? (
              <CircleCheck className="size-5 -me-0.5 shrink-0" style={{ fill: 'var(--callout-color)', color: 'var(--color-fd-card, white)' }} />
            ) : (
              <CircleX className="size-5 -me-0.5 shrink-0" style={{ fill: 'var(--callout-color)', color: 'var(--color-fd-card, white)' }} />
            )}
            <div className="flex flex-col gap-1 min-w-0 flex-1">
              <p className="font-medium my-0!">{toast.valid ? 'Configuration is valid' : 'Validation failed'}</p>
              {!toast.valid && (
                <div className="text-fd-muted-foreground text-xs">
                  <ul className="list-disc list-inside space-y-0.5 m-0 p-0">
                    {toast.errors.map((err, i) => <li key={i}>{err}</li>)}
                  </ul>
                </div>
              )}
            </div>
            <button type="button" onClick={dismissToast} className="rounded-md p-1 self-start text-fd-muted-foreground hover:text-fd-foreground transition-colors">
              <X className="size-4" />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
