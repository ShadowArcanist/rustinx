'use client';

import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { CircleCheck, CircleX, X, Plus, Trash2 } from 'lucide-react';
import { DynamicCodeBlock } from 'fumadocs-ui/components/dynamic-codeblock';
import { buttonVariants } from 'fumadocs-ui/components/ui/button';
import { twMerge } from 'tailwind-merge';

interface RedirectEntry {
  path: string;
  url: string;
  code: string;
}

interface ValidationResult {
  valid: boolean;
  errors: string[];
}

function generateToml(redirects: RedirectEntry[]): string {
  if (redirects.length === 0) return '# Add redirects above';

  const lines: string[] = [];
  for (const r of redirects) {
    if (!r.path.trim() && !r.url.trim()) continue;
    if (lines.length > 0) lines.push('');
    lines.push('[[redirect]]');
    lines.push(`path = "${r.path}"`);
    lines.push(`url = "${r.url}"`);
    if (r.code !== '301') lines.push(`code = ${r.code}`);
  }

  return lines.length > 0 ? lines.join('\n') : '# Add redirects above';
}

function validate(redirects: RedirectEntry[]): ValidationResult {
  const errors: string[] = [];

  if (redirects.length === 0) {
    errors.push('Add at least one redirect');
    return { valid: false, errors };
  }

  const paths = new Set<string>();
  for (let i = 0; i < redirects.length; i++) {
    const r = redirects[i];
    if (!r.path.trim()) errors.push(`Redirect ${i + 1}: path is required`);
    else if (!r.path.startsWith('/'))
      errors.push(`Redirect ${i + 1}: path must start with /`);
    if (!r.url.trim()) errors.push(`Redirect ${i + 1}: URL is required`);
    else if (!r.url.startsWith('https://') && !r.url.startsWith('/'))
      errors.push(`Redirect ${i + 1}: URL must be https:// or /path`);
    if (r.path.trim() && paths.has(r.path))
      errors.push(`Redirect ${i + 1}: duplicate path "${r.path}"`);
    paths.add(r.path);
  }

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

export function RedirectsGenerator() {
  const [redirects, setRedirects] = useState<RedirectEntry[]>([
    { path: '/social/youtube', url: 'https://www.youtube.com/@myuser', code: '301' },
    { path: '/social/github', url: 'https://github.com/myuser', code: '301' },
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

  const updateRedirect = (idx: number, field: keyof RedirectEntry, value: string) =>
    setRedirects((prev) => prev.map((r, i) => (i === idx ? { ...r, [field]: value } : r)));

  const addRedirect = () =>
    setRedirects((prev) => [...prev, { path: '', url: '', code: '301' }]);

  const removeRedirect = (idx: number) =>
    setRedirects((prev) => prev.filter((_, i) => i !== idx));

  const output = useMemo(() => generateToml(redirects), [redirects]);

  return (
    <div className="flex flex-col gap-6 not-prose">
      {/* Redirects */}
      <div className="rounded-xl border border-fd-border bg-fd-card p-5">
        <h3 className="text-lg font-semibold text-fd-card-foreground m-0 mb-4">
          Redirect Rules
        </h3>
        <div className="flex flex-col gap-3">
          {redirects.map((r, i) => (
            <div key={i} className="rounded-lg border border-fd-border bg-fd-background p-4">
              <div className="grid grid-cols-1 sm:grid-cols-[1fr_2fr_5rem_2rem] gap-3 items-end">
                <div className="flex flex-col gap-1.5">
                  <label className={labelCls}>Path</label>
                  <input
                    type="text"
                    value={r.path}
                    onChange={(e) => updateRedirect(i, 'path', e.target.value)}
                    placeholder="/social/youtube"
                    className={monoInputCls}
                  />
                </div>
                <div className="flex flex-col gap-1.5">
                  <label className={labelCls}>Redirect URL</label>
                  <input
                    type="text"
                    value={r.url}
                    onChange={(e) => updateRedirect(i, 'url', e.target.value)}
                    placeholder="https://youtube.com/@user"
                    className={monoInputCls}
                  />
                </div>
                <div className="flex flex-col gap-1.5">
                  <label className={labelCls}>Code</label>
                  <select
                    value={r.code}
                    onChange={(e) => updateRedirect(i, 'code', e.target.value)}
                    className={selectCls}
                    style={selectStyle}
                  >
                    <option value="301">301</option>
                    <option value="302">302</option>
                    <option value="307">307</option>
                    <option value="308">308</option>
                  </select>
                </div>
                <button
                  type="button"
                  onClick={() => removeRedirect(i)}
                  className="text-red-500 hover:text-red-400 p-1.5 self-end"
                >
                  <Trash2 size={16} />
                </button>
              </div>
            </div>
          ))}
        </div>

        <button
          type="button"
          onClick={addRedirect}
          className="mt-3 flex items-center gap-1 text-xs text-fd-muted-foreground hover:text-fd-foreground"
        >
          <Plus size={14} /> Add Redirect
        </button>
      </div>

      {/* Output */}
      <div className="rounded-xl border border-fd-border overflow-hidden">
        <div className="flex items-center border-b border-fd-border bg-fd-muted/50 px-4 py-2">
          <span className="text-sm font-medium text-fd-card-foreground">redirects.toml</span>
          <div className="ml-auto">
            <button
              type="button"
              onClick={() => showToast(validate(redirects))}
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
              <p className="font-medium my-0!">{toast.valid ? 'All redirects are valid' : 'Validation failed'}</p>
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
