/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Background colors - derived from logo's waveform tones
        background: {
          DEFAULT: '#0B0E14',      // Deepest layer - app background
          secondary: '#12161F',    // Cards, elevated surfaces
          tertiary: '#1A1F2B',     // Hover states, borders
          elevated: '#222836',     // Modal backgrounds, dropdowns
        },
        // Waveform accent palette - derived from logo
        accent: {
          dark: '#2A3441',         // Darkest wave layer
          DEFAULT: '#3D4A5C',      // Middle wave tones
          light: '#5A6A7D',        // Lightest wave highlights
          glow: '#7A8A9D',         // Hover glows, focus rings
        },
        // Primary interactive colors
        primary: {
          DEFAULT: '#6B7B8F',      // Primary buttons, links
          hover: '#8494A8',        // Hover state
          active: '#5A6A7D',       // Active/pressed state
        },
        // Navy accent - primary accent for play buttons, CTAs
        navy: {
          DEFAULT: '#2563EB',
          hover: '#3B82F6',
          active: '#1D4ED8',
          muted: '#1E40AF',
          glow: 'rgba(37, 99, 235, 0.4)',
          soft: 'rgba(37, 99, 235, 0.15)',
        },
        // Mint accent - secondary accent for success, liked, now playing
        mint: {
          DEFAULT: '#10B981',
          hover: '#34D399',
          active: '#059669',
          muted: '#047857',
          glow: 'rgba(16, 185, 129, 0.4)',
          soft: 'rgba(16, 185, 129, 0.15)',
        },
        // Text hierarchy
        text: {
          primary: '#FFFFFF',      // Headlines, primary content
          secondary: '#A1A9B4',    // Body text, descriptions
          muted: '#6B7280',        // Captions, timestamps
          disabled: '#4A5260',     // Disabled states
        },
        // Semantic colors - muted/sophisticated
        success: {
          DEFAULT: '#3D6B6B',
          text: '#7DAFAF',
        },
        warning: {
          DEFAULT: '#6B5A3D',
          text: '#C4A66B',
        },
        error: {
          DEFAULT: '#5A3D4A',
          text: '#C47D8F',
        },
        info: {
          DEFAULT: '#3D5A6B',
          text: '#7DA4B8',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
        display: ['DM Serif Display', 'Georgia', 'serif'],
      },
      fontSize: {
        'display-xl': ['3rem', { lineHeight: '1.1', fontWeight: '400' }],
        'display': ['2.25rem', { lineHeight: '1.2', fontWeight: '400' }],
        'overline': ['0.6875rem', { lineHeight: '1.3', fontWeight: '600', letterSpacing: '0.05em' }],
      },
      borderRadius: {
        sm: '4px',
        DEFAULT: '8px',
        lg: '12px',
        xl: '16px',
      },
      transitionDuration: {
        quick: '150ms',
        standard: '200ms',
        emphasis: '300ms',
      },
      animation: {
        'fade-in': 'fadeIn 200ms ease-out',
        'slide-up': 'slideUp 200ms ease-out',
        'scale-in': 'scaleIn 200ms ease-out',
        'wave-pulse': 'wavePulse 1s ease-in-out infinite',
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0' },
          '100%': { opacity: '1' },
        },
        slideUp: {
          '0%': { opacity: '0', transform: 'translateY(8px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        scaleIn: {
          '0%': { opacity: '0', transform: 'scale(0.95)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
        wavePulse: {
          '0%, 100%': { transform: 'scaleY(0.5)' },
          '50%': { transform: 'scaleY(1)' },
        },
      },
      backdropBlur: {
        xs: '4px',
      },
      ringColor: {
        navy: '#2563EB',
        mint: '#10B981',
      },
    },
  },
  plugins: [],
}
