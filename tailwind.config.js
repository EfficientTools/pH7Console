/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        'mono': ['SF Mono', 'Monaco', 'Inconsolata', 'Roboto Mono', 'Courier New', 'monospace'],
      },
      colors: {
        terminal: {
          bg: '#0C0E13',
          surface: '#151922',
          border: '#262B36',
          text: '#E8EAED',
          muted: '#9AA0A6',
          accent: '#4285F4',
          success: '#34A853',
          warning: '#FBBC05',
          error: '#EA4335',
        },
        ai: {
          primary: '#6366F1',
          secondary: '#8B5CF6',
          gradient: 'linear-gradient(135deg, #6366F1 0%, #8B5CF6 100%)',
        }
      },
      animation: {
        'pulse-soft': 'pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'bounce-subtle': 'bounce 1s infinite',
        'typing': 'typing 1.5s steps(40, end)',
      },
      keyframes: {
        typing: {
          'from': { width: '0' },
          'to': { width: '100%' }
        }
      }
    },
  },
  plugins: [],
}
