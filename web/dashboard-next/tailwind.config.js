/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  safelist: [
    'bg-gradient-to-br',
    'from-rose-500/95', 'to-red-800/95', 'shadow-rose-500/25',
    'from-orange-500/95', 'to-rose-700/95', 'shadow-orange-500/25',
    'from-pink-500/95', 'to-fuchsia-800/95', 'shadow-pink-500/25',
    'from-orange-500', 'via-orange-600', 'to-red-800', 'to-amber-600',
    'text-rose-100', 'text-orange-100', 'text-pink-100', 'text-amber-100',
    'lg:w-[55%]', 'lg:w-[58%]',
  ],
  theme: {
    extend: {},
  },
  plugins: [],
};
