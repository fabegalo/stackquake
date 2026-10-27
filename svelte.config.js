import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    // CSS dos componentes vai para um arquivo .css, nunca para <style> inline,
    // para que a CSP possa usar style-src 'self' sem 'unsafe-inline'.
    css: "external",
  },
};
