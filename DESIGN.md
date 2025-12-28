# Resonance Design System

> The source of truth for Resonance's visual identity. This document guides AI agents, developers, designers, and contributors in creating a cohesive, premium music streaming experience.

---

## Table of Contents

1. [Philosophy & Inspiration](#philosophy--inspiration)
2. [Brand Assets](#brand-assets)
3. [Color System](#color-system)
4. [Typography](#typography)
5. [Spacing & Layout](#spacing--layout)
6. [Visual Effects](#visual-effects)
7. [Iconography](#iconography)
8. [Components](#components)
9. [Animation & Motion](#animation--motion)
10. [Accessibility](#accessibility)
11. [Voice & Tone](#voice--tone)
12. [Layout Diagrams](#layout-diagrams)
13. [Custom Theming](#custom-theming)

---

## Philosophy & Inspiration

### Design Principles

Resonance embodies **sophisticated minimalism** with **audiophile credibility**. The interface should feel premium, confident, and purposeful—never cluttered or apologetic.

1. **Confident Restraint** — Bold when it matters, invisible when it doesn't
2. **Audio-First** — Every visual choice reinforces the listening experience
3. **Accessible Luxury** — Premium feel without gatekeeping usability
4. **Honest Materials** — Glass, depth, and light used authentically

### Inspiration Sources

| Reference | What We Take |
|-----------|--------------|
| **dbrand** | Unapologetic boldness, stark contrast, confident voice |
| **Spotify** | Layout patterns, persistent player, navigation structure |
| **Tidal** | Audiophile aesthetic, quality indicators, editorial feel |
| **Nothing / Teenage Engineering** | Technical minimalism, stark typography, intentional constraints |

### The Resonance Mood

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│   Sophisticated ←────────────────────────→ Playful         │
│                        [■■■■░░░░░░]                         │
│                                                             │
│   Minimal ←──────────────────────────────→ Decorated       │
│                        [■■■░░░░░░░]                         │
│                                                             │
│   Technical ←────────────────────────────→ Organic         │
│                        [■■■■■░░░░░]                         │
│                                                             │
│   Dark ←─────────────────────────────────→ Light           │
│                        [■■░░░░░░░░]                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Brand Assets

### Logo

The Resonance logo features **layered audio waveforms** with subtle 3D depth, rendered in graduating gray-blue tones against a dark rounded-square container.

- **Location:** `/docs/assets/logo.png`
- **Dimensions:** 512×512px
- **Clear space:** Minimum 16px on all sides
- **Minimum size:** 32px (favicon), 48px (UI usage)

**Key Visual Elements:**
- Overlapping sine waves suggesting resonance/harmony
- Depth through layered translucency
- Rounded container (radius ~80px at 512px scale)
- Color gradient from deep charcoal to silver-gray

### Wordmark

The wordmark displays "resonance" in a **bold, rounded geometric sans-serif** in dark navy blue.

- **Location:** `/docs/assets/wordmark.png`
- **Base Color:** Navy blue (#1B2838)
- **Typography:** Rounded geometric sans-serif, bold weight
- **Casing:** Always lowercase — never capitalize "Resonance" in the wordmark or text

### Usage Guidelines

```
✓ Logo alone (icon contexts)
✓ Wordmark alone (header contexts)
✓ Logo + Wordmark horizontal lockup
✓ Wordmark color inversion for dark backgrounds (CSS filter)
✓ Logo glow effects for emphasis
✗ Don't change logo colors (glow only)
✗ Don't stretch or distort
✗ Don't use on busy backgrounds without contrast
```

### CRITICAL: Always Use Official Assets

> **IMPORTANT:** The UI must ALWAYS use the official logo (`/logo.png`) and wordmark (`/wordmark.png`) images. Never substitute with placeholder icons, text-only representations, or recreated versions.

**Wordmark Contrast Adaptation:**
The wordmark source is dark navy blue. On dark backgrounds, use CSS filters to invert for visibility:

```tsx
// Dark background (default app theme) - invert wordmark to white
<img
  src="/wordmark.png"
  alt="resonance"
  className="h-6 brightness-0 invert opacity-90"
/>

// Light background - use wordmark as-is (no filter needed)
<img src="/wordmark.png" alt="resonance" className="h-6" />
```

**Logo Glow Effects:**
The logo may receive glow effects for visual emphasis. Never alter the logo's actual colors.

```tsx
// Sidebar logo with hover glow
<img
  src="/logo.png"
  alt="resonance logo"
  className="h-10 w-10 rounded-xl hover:shadow-[0_0_20px_rgba(90,106,125,0.4)] transition-shadow"
/>

// Auth pages - static glow
<img
  src="/logo.png"
  alt="resonance logo"
  className="h-16 w-16 rounded-xl shadow-[0_0_30px_rgba(90,106,125,0.3)]"
/>
```

**Sizing Guidelines:**
- Sidebar: Logo 40px, Wordmark height 20px
- Auth pages: Logo 64px, Wordmark height 28px
- Favicon: 32px (logo only)
- Ensure wordmark fits the container — scale down if needed

**Required Implementation Patterns:**
```tsx
// Sidebar/Navigation - horizontal lockup
<div className="flex items-center gap-3 px-4 py-5">
  <img
    src="/logo.png"
    alt="resonance logo"
    className="h-10 w-10 rounded-xl hover:shadow-[0_0_20px_rgba(90,106,125,0.4)] transition-shadow"
  />
  <img
    src="/wordmark.png"
    alt="resonance"
    className="h-5 brightness-0 invert opacity-90"
  />
</div>

// Auth pages - vertical stacked lockup
<div className="flex flex-col items-center gap-4">
  <img
    src="/logo.png"
    alt="resonance logo"
    className="h-16 w-16 rounded-xl shadow-[0_0_30px_rgba(90,106,125,0.3)]"
  />
  <img
    src="/wordmark.png"
    alt="resonance"
    className="h-7 brightness-0 invert opacity-90"
  />
</div>
```

**Asset Locations (copy to `/apps/web/public/`):**
- Logo: `/docs/assets/logo.png` → `/public/logo.png`
- Wordmark: `/docs/assets/wordmark.png` → `/public/wordmark.png`

**Never Do:**
- Use a Music icon or generic icon as logo placeholder
- Render "resonance" as styled text instead of the wordmark image
- Recreate the logo with CSS gradients or SVG approximations
- Change the logo's colors (glow effects only)
- Capitalize "resonance" — always lowercase

---

## Color System

### Philosophy

Colors are **derived from the logo's waveform tones**—a sophisticated palette of deep charcoals, slate grays, and cool blues. The waveform accent palette provides subtle, sophisticated tones, while **navy blue** serves as the energetic accent color for interactive elements like play buttons and highlights.

### Core Palette

#### Backgrounds
```css
--bg-primary: #0B0E14;      /* Deepest layer - app background */
--bg-secondary: #12161F;    /* Cards, elevated surfaces */
--bg-tertiary: #1A1F2B;     /* Hover states, borders */
--bg-elevated: #222836;     /* Modal backgrounds, dropdowns */
```

#### Waveform Accent Palette (derived from logo)
```css
--accent-dark: #2A3441;     /* Darkest wave layer */
--accent-mid: #3D4A5C;      /* Middle wave tones */
--accent-light: #5A6A7D;    /* Lightest wave highlights */
--accent-glow: #7A8A9D;     /* Hover glows, focus rings */
```

#### Primary Interactive
```css
--primary: #6B7B8F;         /* Primary buttons, links */
--primary-hover: #8494A8;   /* Hover state */
--primary-active: #5A6A7D;  /* Active/pressed state */
```

#### Navy Accent (Primary - Energy & Playback)
```css
--navy: #2563EB;            /* Primary navy - play buttons, CTAs */
--navy-hover: #3B82F6;      /* Lighter on hover */
--navy-active: #1D4ED8;     /* Darker when pressed */
--navy-muted: #1E40AF;      /* Subtle/disabled navy */
--navy-glow: rgba(37, 99, 235, 0.4);   /* Glow effects */
--navy-soft: rgba(37, 99, 235, 0.15);  /* Subtle backgrounds */
```

**Navy Usage (Primary Accent):**
- Play buttons (main call-to-action)
- Track list hover play icons
- Album/playlist card hover glow effects
- Info badges and indicators

#### Mint Accent (Secondary - Success & Liked)
```css
--mint: #10B981;            /* Secondary mint - liked, success */
--mint-hover: #34D399;      /* Lighter on hover */
--mint-active: #059669;     /* Darker when pressed */
--mint-muted: #047857;      /* Subtle/disabled mint */
--mint-glow: rgba(16, 185, 129, 0.4);   /* Glow effects */
--mint-soft: rgba(16, 185, 129, 0.15);  /* Subtle backgrounds */
```

**Mint Usage (Secondary Accent - Use Sparingly):**
- Liked/favorite heart icons (on hover and active)
- Hi-res quality badges
- Success states and confirmations
- "Now playing" indicator
- NOT for primary actions (those use navy)

#### Text Hierarchy
```css
--text-primary: #FFFFFF;    /* Headlines, primary content */
--text-secondary: #A1A9B4;  /* Body text, descriptions */
--text-muted: #6B7280;      /* Captions, timestamps */
--text-disabled: #4A5260;   /* Disabled states */
```

#### Semantic Colors (muted/sophisticated)
```css
/* Success - muted teal */
--success: #3D6B6B;
--success-text: #7DAFAF;

/* Warning - muted amber */
--warning: #6B5A3D;
--warning-text: #C4A66B;

/* Error - muted rose */
--error: #5A3D4A;
--error-text: #C47D8F;

/* Info - accent blue */
--info: #3D5A6B;
--info-text: #7DA4B8;
```

### Tailwind Configuration

```javascript
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      colors: {
        background: {
          DEFAULT: '#0B0E14',
          secondary: '#12161F',
          tertiary: '#1A1F2B',
          elevated: '#222836',
        },
        accent: {
          dark: '#2A3441',
          DEFAULT: '#3D4A5C',
          light: '#5A6A7D',
          glow: '#7A8A9D',
        },
        primary: {
          DEFAULT: '#6B7B8F',
          hover: '#8494A8',
          active: '#5A6A7D',
        },
        // Navy accent - primary accent for playback
        navy: {
          DEFAULT: '#2563EB',
          hover: '#3B82F6',
          active: '#1D4ED8',
          muted: '#1E40AF',
          glow: 'rgba(37, 99, 235, 0.4)',
          soft: 'rgba(37, 99, 235, 0.15)',
        },
        // Mint accent - secondary accent for liked/success
        mint: {
          DEFAULT: '#10B981',
          hover: '#34D399',
          active: '#059669',
          muted: '#047857',
          glow: 'rgba(16, 185, 129, 0.4)',
          soft: 'rgba(16, 185, 129, 0.15)',
        },
        text: {
          primary: '#FFFFFF',
          secondary: '#A1A9B4',
          muted: '#6B7280',
          disabled: '#4A5260',
        },
      },
    },
  },
};
```

### Accessibility Matrix

| Combination | Contrast Ratio | WCAG AA | WCAG AAA |
|-------------|---------------|---------|----------|
| text-primary on bg-primary | 16.5:1 | ✓ | ✓ |
| text-secondary on bg-primary | 8.2:1 | ✓ | ✓ |
| text-muted on bg-primary | 4.8:1 | ✓ | ✗ |
| primary on bg-primary | 5.1:1 | ✓ | ✗ |
| text-primary on bg-secondary | 14.8:1 | ✓ | ✓ |
| accent-light on bg-primary | 4.6:1 | ✓ | ✗ |

> All interactive elements must meet WCAG AA (4.5:1 for normal text, 3:1 for large text)

---

## Typography

### Font Stack

```css
/* Primary - Sharp Sans for Body */
--font-sans: 'Inter', system-ui, -apple-system, sans-serif;

/* Display - Geometric Serif for Accents */
--font-display: 'DM Serif Display', 'Playfair Display', Georgia, serif;
```

### Type Scale

| Name | Size | Weight | Line Height | Usage |
|------|------|--------|-------------|-------|
| `display-xl` | 48px | 400 (serif) | 1.1 | Hero headlines |
| `display` | 36px | 400 (serif) | 1.2 | Section titles |
| `heading-lg` | 24px | 600 | 1.3 | Page titles |
| `heading` | 20px | 600 | 1.3 | Card titles |
| `heading-sm` | 16px | 600 | 1.4 | Subsections |
| `body-lg` | 16px | 400 | 1.6 | Primary body |
| `body` | 14px | 400 | 1.5 | Default body |
| `body-sm` | 13px | 400 | 1.5 | Secondary info |
| `caption` | 12px | 500 | 1.4 | Labels, timestamps |
| `overline` | 11px | 600 | 1.3 | Category labels |

### Serif Accent Usage

The geometric serif (DM Serif Display) should be used sparingly for premium moments:

```
✓ Album titles on detail pages
✓ Artist names in hero sections
✓ "Now Playing" track title
✓ Empty state headlines
✓ Marketing/editorial content

✗ Navigation items
✗ Button labels
✗ Form labels
✗ System messages
✗ Metadata (duration, track count)
```

### Tailwind Typography

```javascript
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        display: ['DM Serif Display', 'Georgia', 'serif'],
      },
      fontSize: {
        'display-xl': ['3rem', { lineHeight: '1.1', fontWeight: '400' }],
        'display': ['2.25rem', { lineHeight: '1.2', fontWeight: '400' }],
      },
    },
  },
};
```

---

## Spacing & Layout

### Spacing Scale

Based on 4px base unit:

| Token | Value | Usage |
|-------|-------|-------|
| `space-0` | 0px | Reset |
| `space-1` | 4px | Tight inline spacing |
| `space-2` | 8px | Icon gaps, compact padding |
| `space-3` | 12px | Default component padding |
| `space-4` | 16px | Card padding, section gaps |
| `space-5` | 20px | Medium sections |
| `space-6` | 24px | Large sections |
| `space-8` | 32px | Page sections |
| `space-10` | 40px | Major divisions |
| `space-12` | 48px | Hero spacing |
| `space-16` | 64px | Page margins |

### Grid System

```css
/* Content max-width */
--content-max: 1400px;

/* Grid columns */
--grid-cols-mobile: 2;
--grid-cols-tablet: 3;
--grid-cols-desktop: 5;
--grid-cols-wide: 6;

/* Grid gap */
--grid-gap: 16px;
--grid-gap-lg: 24px;
```

### Breakpoints

| Name | Width | Columns | Sidebar |
|------|-------|---------|---------|
| `sm` | 640px | 2 | Hidden |
| `md` | 768px | 3 | Hidden |
| `lg` | 1024px | 4 | Collapsed |
| `xl` | 1280px | 5 | Expanded |
| `2xl` | 1536px | 6 | Expanded |

### Layout Density

Balanced density—comfortable without wasting space:

```css
/* Card dimensions */
--card-min-width: 160px;
--card-max-width: 220px;
--card-aspect-ratio: 1 / 1; /* Album art */

/* List item height */
--list-item-height: 56px;
--list-item-compact: 48px;

/* Touch targets */
--touch-target-min: 44px;
```

---

## Visual Effects

### Glassmorphism

The signature Resonance effect—frosted glass surfaces that create depth and sophistication.

```css
/* Standard glass card */
.glass {
  background: rgba(18, 22, 31, 0.7);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.05);
}

/* Elevated glass (modals, dropdowns) */
.glass-elevated {
  background: rgba(34, 40, 54, 0.85);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

/* Subtle glass (nav bar, player) */
.glass-subtle {
  background: rgba(11, 14, 20, 0.8);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
}
```

### Wave Motif Usage

The waveform from the logo appears throughout the UI:

| Context | Implementation |
|---------|----------------|
| **Audio visualizer** | Real-time animated waveform responding to music |
| **Background texture** | Subtle, low-opacity repeating wave pattern |
| **Loading states** | Animated wave pulse |
| **Empty states** | Decorative wave illustration |

```css
/* Background wave texture */
.wave-texture {
  background-image: url('/assets/wave-pattern.svg');
  background-size: 200px auto;
  background-repeat: repeat-x;
  opacity: 0.03;
}
```

### Corner Radius

Subtle rounding that echoes the logo container:

| Token | Value | Usage |
|-------|-------|-------|
| `radius-sm` | 4px | Badges, small buttons |
| `radius` | 8px | Cards, inputs, buttons |
| `radius-lg` | 12px | Modals, large cards |
| `radius-xl` | 16px | Hero sections |
| `radius-full` | 9999px | Pills, avatars, circular buttons |

### Glow Effects

Subtle glows for interactive feedback:

```css
/* Focus glow */
.focus-glow {
  box-shadow: 0 0 0 3px rgba(107, 123, 143, 0.3);
}

/* Hover glow (album art) */
.hover-glow {
  box-shadow: 0 0 20px rgba(90, 106, 125, 0.4);
}

/* Active accent glow */
.accent-glow {
  box-shadow: 0 0 30px rgba(122, 138, 157, 0.3);
}
```

---

## Iconography

### Style Guidelines

Use **outlined/stroke icons** with consistent weight:

- **Stroke width:** 1.5px - 2px
- **Style:** Rounded line caps and joins
- **Size:** 20px default, 16px compact, 24px emphasis
- **Color:** Inherits from text color

### Recommended Icon Set

[Lucide Icons](https://lucide.dev) as the primary icon library—clean, consistent, MIT licensed.

```jsx
import { Play, Pause, SkipForward, Heart, Search } from 'lucide-react';

<Play size={20} strokeWidth={2} />
```

### Common Icons

| Action | Icon | Notes |
|--------|------|-------|
| Play | `play` | Filled when active |
| Pause | `pause` | |
| Skip | `skip-forward` / `skip-back` | |
| Shuffle | `shuffle` | Accent color when active |
| Repeat | `repeat` / `repeat-1` | |
| Volume | `volume-2` / `volume-x` | |
| Heart/Like | `heart` | Filled when liked |
| Add to playlist | `plus` | |
| More options | `more-horizontal` | |
| Search | `search` | |
| Library | `library` | |
| Home | `home` | |
| Settings | `settings` | |

---

## Components

### Buttons

```css
/* Primary Button */
.btn-primary {
  @apply px-4 py-2 rounded-lg font-medium
         bg-primary text-white
         hover:bg-primary-hover
         active:bg-primary-active
         focus:outline-none focus:ring-2 focus:ring-accent-glow
         transition-all duration-150;
}

/* Accent Button (navy - for play actions) */
.btn-accent {
  @apply px-4 py-2 rounded-lg font-medium
         bg-navy text-white
         hover:bg-navy-hover
         active:bg-navy-active
         shadow-[0_0_20px_rgba(37,99,235,0.3)]
         hover:shadow-[0_0_25px_rgba(37,99,235,0.4)]
         focus:outline-none focus:ring-2 focus:ring-navy-glow
         transition-all duration-150;
}

/* Secondary Button (glass) */
.btn-secondary {
  @apply px-4 py-2 rounded-lg font-medium
         bg-background-tertiary/50 text-text-primary
         backdrop-blur-sm border border-white/5
         hover:bg-background-elevated hover:border-white/10
         focus:outline-none focus:ring-2 focus:ring-accent-glow
         transition-all duration-150;
}

/* Ghost Button */
.btn-ghost {
  @apply px-4 py-2 rounded-lg font-medium
         text-text-secondary
         hover:bg-background-tertiary hover:text-text-primary
         focus:outline-none focus:ring-2 focus:ring-accent-glow
         transition-all duration-150;
}

/* Icon Button */
.btn-icon {
  @apply p-2 rounded-lg
         text-text-secondary
         hover:bg-background-tertiary hover:text-text-primary
         focus:outline-none focus:ring-2 focus:ring-accent-glow
         transition-all duration-150;
}
```

### Cards

```css
/* Album/Playlist Card */
.card {
  @apply rounded-lg overflow-hidden
         bg-background-secondary
         border border-white/5
         hover:bg-background-tertiary
         hover:border-accent-dark
         transition-all duration-150;
}

/* Glass Card */
.card-glass {
  @apply rounded-lg overflow-hidden p-4
         bg-background-secondary/70
         backdrop-blur-xl
         border border-white/5;
}
```

### Album Artwork

```css
/* Album art container */
.album-art {
  @apply relative aspect-square rounded-lg overflow-hidden
         bg-background-tertiary;
}

/* Hover overlay */
.album-art-overlay {
  @apply absolute inset-0 flex items-center justify-center
         bg-black/40 opacity-0
         hover:opacity-100
         transition-opacity duration-150;
}

/* Hover glow border */
.album-art:hover {
  @apply shadow-[0_0_20px_rgba(90,106,125,0.4)];
}
```

**Hover Behavior:**
1. Overlay dims artwork (40% black)
2. Play button appears centered
3. Subtle glow border emerges
4. Track/album info can appear at bottom

### Inputs

```css
/* Text Input */
.input {
  @apply w-full px-3 py-2 rounded-lg
         bg-background-secondary
         border border-background-tertiary
         text-text-primary placeholder:text-text-muted
         focus:outline-none focus:ring-2 focus:ring-accent-glow
         focus:border-accent-light
         transition-all duration-150;
}

/* Search Input */
.input-search {
  @apply input pl-10; /* Space for search icon */
}
```

### Navigation Sidebar

```css
/* Sidebar container */
.sidebar {
  @apply fixed left-0 top-0 bottom-0
         w-64 bg-background/95
         backdrop-blur-xl
         border-r border-white/5
         flex flex-col;
}

/* Nav item */
.nav-item {
  @apply flex items-center gap-3 px-4 py-3
         text-text-secondary font-medium
         hover:text-text-primary hover:bg-background-tertiary
         rounded-lg mx-2
         transition-all duration-150;
}

.nav-item-active {
  @apply text-text-primary bg-background-tertiary;
}
```

### Player Bar

```css
/* Persistent player bar */
.player-bar {
  @apply fixed bottom-0 left-0 right-0 h-20
         bg-background/95
         backdrop-blur-xl
         border-t border-white/5
         flex items-center px-4
         z-50;
}

/* Progress bar (wave-inspired) */
.progress-track {
  @apply h-1 bg-background-tertiary rounded-full;
}

.progress-fill {
  @apply h-full bg-accent-light rounded-full
         transition-all duration-100;
}
```

### Quality Badges

Simple, understated quality indicators:

```css
/* Quality badge */
.badge-quality {
  @apply inline-flex items-center px-1.5 py-0.5
         text-[10px] font-semibold uppercase tracking-wider
         rounded bg-accent-dark/50 text-text-secondary
         border border-white/5;
}
```

| Format | Label | Color Variant |
|--------|-------|---------------|
| FLAC | `FLAC` | Default |
| Hi-Res | `HI-RES` | Accent glow border |
| Lossless | `LOSSLESS` | Default |
| MP3/AAC | `MP3` / `AAC` | Muted |

---

## Animation & Motion

### Timing Principles

- **Quick interactions:** 150ms (hovers, toggles)
- **Standard transitions:** 200ms (page elements, reveals)
- **Emphasis animations:** 300ms (modals, major state changes)
- **Never exceed:** 400ms for UI animations

### Easing Functions

```css
/* Standard ease - most interactions */
--ease-standard: cubic-bezier(0.4, 0, 0.2, 1);

/* Ease out - entering elements */
--ease-out: cubic-bezier(0, 0, 0.2, 1);

/* Ease in - exiting elements */
--ease-in: cubic-bezier(0.4, 0, 1, 1);

/* Spring - playful emphasis */
--ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
```

### Standard Animations

```css
/* Fade in */
@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

/* Slide up (for toasts, cards) */
@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Scale in (for modals) */
@keyframes scaleIn {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

/* Wave pulse (loading) */
@keyframes wavePulse {
  0%, 100% { transform: scaleY(0.5); }
  50% { transform: scaleY(1); }
}
```

### Tailwind Animation Classes

```javascript
// tailwind.config.js
module.exports = {
  theme: {
    extend: {
      animation: {
        'fade-in': 'fadeIn 200ms ease-out',
        'slide-up': 'slideUp 200ms ease-out',
        'scale-in': 'scaleIn 200ms ease-out',
        'wave-pulse': 'wavePulse 1s ease-in-out infinite',
      },
    },
  },
};
```

### Motion Guidelines

```
✓ Subtle scale on card hover (1.02x)
✓ Color transitions on interactive elements
✓ Glow fade-in on focus
✓ Smooth progress bar updates
✓ Staggered list item entrance

✗ Bouncy/elastic animations
✗ Long delays before interaction
✗ Animations that block user action
✗ Motion for motion's sake
```

---

## Accessibility

### Color & Contrast

- All text meets **WCAG AA** (4.5:1 for normal, 3:1 for large)
- Interactive elements have **3:1** contrast against backgrounds
- Don't rely on color alone—use icons, patterns, or text labels
- Test with color blindness simulators (protanopia, deuteranopia)

### Focus Management

```css
/* Visible focus for keyboard navigation */
*:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px rgba(107, 123, 143, 0.5);
}

/* Remove focus ring for mouse users */
*:focus:not(:focus-visible) {
  outline: none;
  box-shadow: none;
}
```

### Keyboard Navigation

| Key | Action |
|-----|--------|
| `Tab` | Move to next interactive element |
| `Shift + Tab` | Move to previous element |
| `Enter` / `Space` | Activate buttons, links |
| `Arrow Keys` | Navigate within components (menus, sliders) |
| `Escape` | Close modals, dropdowns |
| `Space` | Play/pause (global) |
| `←` / `→` | Seek (when player focused) |

### Screen Reader Considerations

```jsx
/* Announce dynamic content */
<div role="status" aria-live="polite">
  Now playing: {trackName} by {artistName}
</div>

/* Label icon buttons */
<button aria-label="Play">
  <PlayIcon />
</button>

/* Describe album art */
<img
  src={albumArt}
  alt={`Album cover for ${albumName} by ${artistName}`}
/>
```

### Reduced Motion

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    animation-delay: 0ms !important;
    transition-delay: 0ms !important;
  }
}
```

### Accessibility Checklist

- [ ] All interactive elements have visible focus states
- [ ] Color contrast meets WCAG AA
- [ ] Images have meaningful alt text
- [ ] Forms have associated labels
- [ ] Error messages are announced to screen readers
- [ ] Modal focus is trapped appropriately
- [ ] Skip links provided for main content
- [ ] Reduced motion preference respected
- [ ] Touch targets are at least 44×44px
- [ ] Content is readable at 200% zoom

---

## Voice & Tone

### Personality

**Friendly and casual** with confidence. We're knowledgeable about music but never elitist.

### Guidelines

| Context | Tone | Example |
|---------|------|---------|
| Empty states | Helpful, warm | "Your library is waiting. Start exploring." |
| Errors | Calm, solution-focused | "Couldn't load that track. Let's try again." |
| Success | Understated | "Added to your library." |
| Loading | Brief | "Loading..." or silence |
| Onboarding | Encouraging | "Welcome to Resonance. Let's find your sound." |

### Writing Principles

1. **Be concise** — Every word earns its place
2. **Be human** — Write like you talk (professionally)
3. **Be helpful** — Guide users to the next action
4. **Avoid jargon** — Unless it's music terminology users expect

### Examples

```
✓ "No results for 'jazz piano'"
✗ "Your search query returned 0 results"

✓ "Shuffle on"
✗ "Shuffle mode has been enabled"

✓ "Something went wrong. Try again?"
✗ "Error 500: Internal server error"
```

---

## Layout Diagrams

### Desktop Layout (≥1024px)

```
┌─────────────────────────────────────────────────────────────────────┐
│ ┌──────────┐ ┌──────────────────────────────────────────────────────│
│ │          │ │  HEADER (breadcrumb / search)                        │
│ │  SIDEBAR │ ├──────────────────────────────────────────────────────│
│ │          │ │                                                      │
│ │  • Home  │ │  MAIN CONTENT AREA                                   │
│ │  • Search│ │                                                      │
│ │  • Libra │ │  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐            │
│ │          │ │  │     │ │     │ │     │ │     │ │     │            │
│ │ ──────── │ │  │ Art │ │ Art │ │ Art │ │ Art │ │ Art │            │
│ │          │ │  │     │ │     │ │     │ │     │ │     │            │
│ │ Playlists│ │  └─────┘ └─────┘ └─────┘ └─────┘ └─────┘            │
│ │  • Daily │ │  Title    Title   Title   Title   Title              │
│ │  • Chill │ │  Artist   Artist  Artist  Artist  Artist             │
│ │          │ │                                                      │
│ └──────────┘ │                                           (scrolls)  │
│              └──────────────────────────────────────────────────────│
├─────────────────────────────────────────────────────────────────────┤
│ ┌─────┐                     advancement                    vol   ─── │
│ │ Art │  Track Title      ▶ ◀◀ ▶▶ ○                      ▁▂▃▄    Q │
│ └─────┘  Artist Name      ────●───────────────────  2:34 / 4:12     │
├─────────────────────────────────────────────────────────────────────┤
│                           PLAYER BAR (fixed)                        │
└─────────────────────────────────────────────────────────────────────┘
```

### Mobile Layout (<768px)

```
┌─────────────────────────┐
│  ☰  RESONANCE    🔍  ⚙️  │  ← Header
├─────────────────────────┤
│                         │
│  Recently Played        │
│  ┌─────┐ ┌─────┐       │
│  │     │ │     │  →    │  ← Horizontal scroll
│  └─────┘ └─────┘       │
│                         │
│  Made For You           │
│  ┌─────┐ ┌─────┐       │
│  │     │ │     │  →    │
│  └─────┘ └─────┘       │
│                         │
│         (scrolls)       │
├─────────────────────────┤
│ ┌─────┐ Track Title  ▶  │  ← Mini player
│ └─────┘ Artist          │
├─────────────────────────┤
│  🏠    🔍    📚    👤   │  ← Bottom tabs
│ Home  Search Lib  You   │
└─────────────────────────┘
```

### Album Detail Page

```
┌─────────────────────────────────────────────────────────────────────┐
│ ← Back                                                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│     ┌───────────────────┐                                           │
│     │                   │    Album Title (serif)                    │
│     │                   │    Artist Name                            │
│     │    ALBUM ART      │    2023 • 12 songs • 48 min               │
│     │    (large)        │                                           │
│     │                   │    [ ▶ Play ] [ ♡ ] [ ••• ]               │
│     └───────────────────┘                                           │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│  #   TITLE                                      💿  DURATION        │
├─────────────────────────────────────────────────────────────────────┤
│  1   Track One                                 FLAC   3:42          │
│  2   Track Two                                 FLAC   4:18          │
│  3   Track Three                               FLAC   3:55          │
│  ...                                                                │
└─────────────────────────────────────────────────────────────────────┘
```

### Now Playing (Expanded)

```
┌─────────────────────────────────────────────────────────────────────┐
│                           ∨ (collapse)                              │
│                                                                     │
│                    ┌─────────────────────┐                          │
│                    │                     │                          │
│                    │                     │                          │
│                    │     ALBUM ART       │                          │
│                    │      (hero)         │                          │
│                    │                     │                          │
│                    │                     │                          │
│                    └─────────────────────┘                          │
│                                                                     │
│                    Track Title (serif, large)                       │
│                    Artist Name                                      │
│                                                                     │
│              ────────────●──────────────────                        │
│              1:42                      4:18                         │
│                                                                     │
│                    ◀◀    ▶    ▶▶                                   │
│                                                                     │
│                 🔀        ♡        🔁                               │
│                                                                     │
│    ┌─────────────────────────────────────────────────────┐          │
│    │  ≋≋≋≋≋≋≋≋≋≋≋  AUDIO VISUALIZER  ≋≋≋≋≋≋≋≋≋≋≋≋≋≋≋    │          │
│    └─────────────────────────────────────────────────────┘          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Custom Theming

### CSS Variables Reference

Users can override these variables to create custom themes:

```css
:root {
  /* Backgrounds */
  --color-bg-primary: #0B0E14;
  --color-bg-secondary: #12161F;
  --color-bg-tertiary: #1A1F2B;
  --color-bg-elevated: #222836;

  /* Accent */
  --color-accent-dark: #2A3441;
  --color-accent: #3D4A5C;
  --color-accent-light: #5A6A7D;
  --color-accent-glow: #7A8A9D;

  /* Primary (interactive) */
  --color-primary: #6B7B8F;
  --color-primary-hover: #8494A8;
  --color-primary-active: #5A6A7D;

  /* Text */
  --color-text-primary: #FFFFFF;
  --color-text-secondary: #A1A9B4;
  --color-text-muted: #6B7280;
  --color-text-disabled: #4A5260;

  /* Semantic */
  --color-success: #3D6B6B;
  --color-success-text: #7DAFAF;
  --color-warning: #6B5A3D;
  --color-warning-text: #C4A66B;
  --color-error: #5A3D4A;
  --color-error-text: #C47D8F;

  /* Effects */
  --blur-amount: 20px;
  --border-radius: 8px;
  --transition-fast: 150ms;
  --transition-normal: 200ms;
}
```

### Creating a Custom Theme

1. Create a CSS file with variable overrides
2. Import after the main stylesheet
3. Variables cascade automatically

**Example: High Contrast Theme**

```css
/* custom-themes/high-contrast.css */
:root {
  --color-bg-primary: #000000;
  --color-bg-secondary: #0A0A0A;
  --color-text-primary: #FFFFFF;
  --color-text-secondary: #E0E0E0;
  --color-accent-light: #FFFFFF;
}
```

**Example: Warm Theme**

```css
/* custom-themes/warm.css */
:root {
  --color-bg-primary: #1A1512;
  --color-bg-secondary: #241E19;
  --color-accent: #8B6B4A;
  --color-accent-light: #A88B6A;
}
```

---

## Changelog

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2024-XX-XX | Initial design system documentation |

---

*This document is the source of truth for Resonance's visual identity. When in doubt, reference these guidelines. When guidelines conflict with user needs, accessibility wins.*
