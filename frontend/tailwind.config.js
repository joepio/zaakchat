/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        'sans': [
          'RijksoverheidSans',
          'Arial',
          'sans-serif',
        ],
      },
      colors: {
        'bg-primary': 'var(--bg-primary)',
        'bg-secondary': 'var(--bg-secondary)',
        'bg-tertiary': 'var(--bg-tertiary)',
        'bg-hover': 'var(--bg-hover)',
        'bg-accent': 'var(--bg-accent)',
        'bg-success': 'var(--bg-success)',
        'bg-error': 'var(--bg-error)',
        'bg-warning': 'var(--bg-warning)',
        'text-primary': 'var(--text-primary)',
        'text-secondary': 'var(--text-secondary)',
        'text-tertiary': 'var(--text-tertiary)',
        'text-inverse': 'var(--text-inverse)',
        'text-success': 'var(--text-success)',
        'text-error': 'var(--text-error)',
        'text-warning': 'var(--text-warning)',
        'border-primary': 'var(--border-primary)',
        'border-secondary': 'var(--border-secondary)',
        'border-hover': 'var(--border-hover)',
        'border-focus': 'var(--border-focus)',
        'link-primary': 'var(--link-primary)',
        'link-hover': 'var(--link-hover)',
        'link-visited': 'var(--link-visited)',
        'button-primary-bg': 'var(--button-primary-bg)',
        'button-primary-hover': 'var(--button-primary-hover)',
        'button-secondary-bg': 'var(--button-secondary-bg)',
        'button-secondary-hover': 'var(--button-secondary-hover)',
        'button-danger-bg': 'var(--button-danger-bg)',
        'button-danger-hover': 'var(--button-danger-hover)',
        'status-open': 'var(--status-open)',
        'status-progress': 'var(--status-progress)',
        'status-closed': 'var(--status-closed)',
        'overlay': 'var(--overlay)',
        'overlay-light': 'var(--overlay-light)',
      },
      boxShadow: {
        'theme-sm': 'var(--shadow-sm)',
        'theme-md': 'var(--shadow-md)',
        'theme-lg': 'var(--shadow-lg)',
      },
      animation: {
        'timeline-appear': 'timelineItemAppear 0.3s ease-out',
      },
      keyframes: {
        timelineItemAppear: {
          '0%': {
            opacity: '0',
            transform: 'translateY(-10px)',
          },
          '100%': {
            opacity: '1',
            transform: 'translateY(0)',
          },
        },
      },
    },
  },
  plugins: [
    require('@tailwindcss/typography'),
  ],
}
