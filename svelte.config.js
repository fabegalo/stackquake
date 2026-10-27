import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    // Component CSS goes to a .css file, never to inline <style>, so the CSP
    // can use style-src 'self' without 'unsafe-inline'.
    css: "external",
  },
};
