/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'theme-primary': 'var(--theme-primary)',
        'theme-secondary': 'var(--theme-secondary)',
        'theme-purple': 'var(--theme-purple)',
        'theme-green': 'var(--theme-green)',
        'theme-orange': 'var(--theme-orange)',
        'theme-bg': 'var(--theme-bg)',
        'theme-bg-lighter': 'var(--theme-bg-lighter)',
        'theme-bg-subtle': 'var(--theme-bg-subtle)',
        'theme-text': 'var(--theme-text)',
        'theme-text-muted': 'var(--theme-text-muted)',
        'theme-border': 'var(--theme-border)'
      }
    },
  },
  plugins: [],
}