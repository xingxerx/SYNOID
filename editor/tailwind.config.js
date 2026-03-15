/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        crt: {
          bg: '#050505',
          green: '#00FF41',
          'green-dim': '#003B00',
          amber: '#FFB000',
          teal: '#008080',
          orange: '#FF8C00',
          border: '#00FF41',
        },
        synoid: {
          bg: '#050505',
          panel: 'rgba(0, 59, 0, 0.05)',
          sidebar: 'rgba(0, 59, 0, 0.08)',
          surface: 'rgba(0, 255, 65, 0.05)',
          border: '#00FF41',
          'border-light': 'rgba(0, 255, 65, 0.3)',
          orange: '#FF8C00',
          blue: '#008080',
          green: '#00FF41',
          purple: '#b464ff',
          red: '#ff5050',
          yellow: '#FFB000',
          'text-primary': '#00FF41',
          'text-secondary': 'rgba(0, 255, 65, 0.7)',
          'text-dim': 'rgba(0, 255, 65, 0.4)',
        },
      },
      fontFamily: {
        mono: ['"IBM Plex Mono"', 'monospace'],
        sans: ['"IBM Plex Mono"', 'monospace'],
      },
    },
  },
  plugins: [],
};

