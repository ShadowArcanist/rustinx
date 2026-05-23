import defaultMdxComponents from 'fumadocs-ui/mdx';
import { File, Folder, Files } from 'fumadocs-ui/components/files';
import { Tab, Tabs } from 'fumadocs-ui/components/tabs';
import { RustinxConfigurator } from './rustinx-configurator';
import { RedirectsGenerator } from './redirects-generator';
import type { MDXComponents } from 'mdx/types';

export function getMDXComponents(components?: MDXComponents) {
  return {
    ...defaultMdxComponents,
    File,
    Folder,
    Files,
    Tab,
    Tabs,
    RustinxConfigurator,
    RedirectsGenerator,
    ...components,
  } satisfies MDXComponents;
}

export const useMDXComponents = getMDXComponents;

declare global {
  type MDXProvidedComponents = ReturnType<typeof getMDXComponents>;
}
