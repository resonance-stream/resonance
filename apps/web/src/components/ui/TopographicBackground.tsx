import { useEffect, useRef } from 'react';
import { createNoise3D } from 'simplex-noise';

interface TopographicBackgroundProps {
  className?: string;
}

export function TopographicBackground({ className = '' }: TopographicBackgroundProps): JSX.Element {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationRef = useRef<number>(0);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const noise3D = createNoise3D();

    // Configuration - inspired by real topographic maps
    const config = {
      scale: 0.002,          // Noise scale (smaller = larger features)
      speed: 0.00015,        // Animation speed (slower for subtlety)
      contourLevels: 20,     // More contour lines for detail
      majorInterval: 5,      // Every 5th line is a major contour
      baseOpacity: 0.12,     // Base line opacity
      // Color palette - earth tones with accent colors
      colors: [
        { rgb: [139, 90, 43], name: 'sienna' },      // Brown - low elevation
        { rgb: [160, 120, 60], name: 'tan' },        // Tan
        { rgb: [16, 185, 129], name: 'mint' },       // Mint - mid elevation
        { rgb: [34, 139, 134], name: 'teal' },       // Teal
        { rgb: [37, 99, 235], name: 'navy' },        // Navy - high elevation
        { rgb: [88, 80, 140], name: 'purple' },      // Purple accent
      ],
    };

    let time = 0;

    const resize = (): void => {
      const dpr = window.devicePixelRatio || 1;
      canvas.width = window.innerWidth * dpr;
      canvas.height = window.innerHeight * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const drawContours = (width: number, height: number): void => {
      ctx.clearRect(0, 0, width, height);

      const cellSize = 8; // Resolution of the grid
      const cols = Math.ceil(width / cellSize) + 1;
      const rows = Math.ceil(height / cellSize) + 1;

      // Generate noise field
      const field: number[][] = Array.from({ length: rows }, () =>
        Array.from({ length: cols }, () => 0)
      );
      for (let y = 0; y < rows; y++) {
        for (let x = 0; x < cols; x++) {
          const nx = x * cellSize * config.scale;
          const ny = y * cellSize * config.scale;
          field[y]![x] = noise3D(nx, ny, time);
        }
      }

      // Draw contour lines using marching squares
      for (let level = 0; level < config.contourLevels; level++) {
        const threshold = -1 + (2 * level) / (config.contourLevels - 1);

        // Determine if this is a major contour line (every 5th)
        const isMajor = level % config.majorInterval === 0;

        // Color selection based on elevation level with smooth transitions
        const normalizedLevel = level / (config.contourLevels - 1);
        const colorIndex = Math.floor(normalizedLevel * (config.colors.length - 1));
        const nextColorIndex = Math.min(colorIndex + 1, config.colors.length - 1);
        const colorBlend = (normalizedLevel * (config.colors.length - 1)) % 1;

        // Interpolate between adjacent colors for smooth gradients
        const color1 = config.colors[colorIndex]!.rgb;
        const color2 = config.colors[nextColorIndex]!.rgb;
        const blendedColor = [
          Math.round(color1[0]! + (color2[0]! - color1[0]!) * colorBlend),
          Math.round(color1[1]! + (color2[1]! - color1[1]!) * colorBlend),
          Math.round(color1[2]! + (color2[2]! - color1[2]!) * colorBlend),
        ];

        // Vary opacity - major lines more visible, add slight randomness per level
        const opacityVariation = 0.8 + (Math.sin(level * 1.7) * 0.2 + 0.2);
        const opacity = config.baseOpacity * (isMajor ? 1.8 : opacityVariation);

        // Line width - major contours are thicker
        const lineWidth = isMajor ? 1.5 : 0.8;

        ctx.strokeStyle = `rgba(${blendedColor[0]}, ${blendedColor[1]}, ${blendedColor[2]}, ${opacity})`;
        ctx.lineWidth = lineWidth;
        ctx.beginPath();

        // Marching squares algorithm
        for (let y = 0; y < rows - 1; y++) {
          for (let x = 0; x < cols - 1; x++) {
            const rowY = field[y]!;
            const rowY1 = field[y + 1]!;

            const tl = rowY[x]! > threshold ? 1 : 0;
            const tr = rowY[x + 1]! > threshold ? 1 : 0;
            const br = rowY1[x + 1]! > threshold ? 1 : 0;
            const bl = rowY1[x]! > threshold ? 1 : 0;

            const caseIndex = tl * 8 + tr * 4 + br * 2 + bl;

            if (caseIndex === 0 || caseIndex === 15) continue;

            const px = x * cellSize;
            const py = y * cellSize;

            // Interpolate edge positions
            const lerp = (a: number, b: number): number => {
              if (Math.abs(b - a) < 0.0001) return 0.5;
              return (threshold - a) / (b - a);
            };

            const top = px + lerp(rowY[x]!, rowY[x + 1]!) * cellSize;
            const bottom = px + lerp(rowY1[x]!, rowY1[x + 1]!) * cellSize;
            const left = py + lerp(rowY[x]!, rowY1[x]!) * cellSize;
            const right = py + lerp(rowY[x + 1]!, rowY1[x + 1]!) * cellSize;

            // Draw line segments based on case
            const drawLine = (x1: number, y1: number, x2: number, y2: number): void => {
              ctx.moveTo(x1, y1);
              ctx.lineTo(x2, y2);
            };

            switch (caseIndex) {
              case 1: case 14: drawLine(px, left, bottom, py + cellSize); break;
              case 2: case 13: drawLine(bottom, py + cellSize, px + cellSize, right); break;
              case 3: case 12: drawLine(px, left, px + cellSize, right); break;
              case 4: case 11: drawLine(top, py, px + cellSize, right); break;
              case 5:
                drawLine(px, left, top, py);
                drawLine(bottom, py + cellSize, px + cellSize, right);
                break;
              case 6: case 9: drawLine(top, py, bottom, py + cellSize); break;
              case 7: case 8: drawLine(px, left, top, py); break;
              case 10:
                drawLine(top, py, px + cellSize, right);
                drawLine(px, left, bottom, py + cellSize);
                break;
            }
          }
        }

        ctx.stroke();
      }
    };

    const animate = (): void => {
      time += config.speed;
      drawContours(window.innerWidth, window.innerHeight);
      animationRef.current = requestAnimationFrame(animate);
    };

    resize();

    // Respect user preference for reduced motion
    const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (prefersReducedMotion) {
      drawContours(window.innerWidth, window.innerHeight); // Draw one static frame
    } else {
      animate();
    }

    window.addEventListener('resize', resize);

    return () => {
      window.removeEventListener('resize', resize);
      cancelAnimationFrame(animationRef.current);
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      className={`fixed inset-0 w-full h-full pointer-events-none ${className}`}
      style={{ zIndex: -1 }}
      aria-hidden="true"
    />
  );
}
