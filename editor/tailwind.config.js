/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        synoid: {
          bg: '#16161a',
          panel: '#1e1e22',
          sidebar: '#1a1a1e',
          surface: '#252529',
          border: '#2a2a30',
          'border-light': '#3a3a42',
          orange: '#ff7832',
          blue: '#50a0ff',
          green: '#50c878',
          purple: '#b464ff',
          red: '#ff5050',
          yellow: '#ffc040',
          'text-primary': '#dcdcdc',
          'text-secondary': '#8c8c96',
          'text-dim': '#5a5a64',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'Consolas', 'monospace'],
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
};
