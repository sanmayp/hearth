import adapter from '@sveltejs/adapter-static';

/** Static output feeds Tauri, PWA hosting, and any file server. */
const config = {
  kit: {
    adapter: adapter({ fallback: 'index.html' })
  }
};

export default config;
