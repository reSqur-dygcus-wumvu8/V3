/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{svelte,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Couleurs pour le thème sombre
        primary: {
          50: '#f0f9ff',
          100: '#e0f2fe',
          200: '#bae6fd',
          300: '#7dd3fc',
          400: '#38bdf8',
          500: '#0ea5e9',
          600: '#0284c7',
          700: '#0369a1',
          800: '#075985',
          900: '#0c4a6e',
          950: '#082f49',
        },
        secondary: {
          50: '#f8fafc',
          100: '#f1f5f9',
          200: '#e2e8f0',
          300: '#cbd5e1',
          400: '#94a3b8',
          500: '#64748b',
          600: '#475569',
          700: '#334155',
          800: '#1e293b',
          900: '#0f172a',
          950: '#020617',
        },
        // Couleurs pour les cotations OTAN
        reliability: {
          A: '#10b981', // Vert (totalement fiable)
          B: '#3b82f6', // Bleu (généralement fiable)
          C: '#f59e0b', // Jaune (assez fiable)
          D: '#f97316', // Orange (peu fiable)
          E: '#ef4444', // Rouge (fiabilité non évaluable)
          F: '#6b7280', // Gris (non évaluable)
        },
        credibility: {
          1: '#10b981', // Vert (confirmée)
          2: '#3b82f6', // Bleu (probablement vraie)
          3: '#f59e0b', // Jaune (possiblement vraie)
          4: '#f97316', // Orange (douteuse)
          5: '#ef4444', // Rouge (improbable)
          6: '#6b7280', // Gris (non évaluable)
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
      },
    },
  },
  plugins: [],
  darkMode: 'class',
}
