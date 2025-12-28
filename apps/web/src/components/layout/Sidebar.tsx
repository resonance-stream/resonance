import { NavLink } from 'react-router-dom';
import { Home, Search, Library, Settings } from 'lucide-react';
import { cn } from '../../lib/utils';
import { ScrollArea } from '../ui/ScrollArea';

interface NavItemProps {
  to: string;
  icon: React.ReactNode;
  label: string;
}

function NavItem({ to, icon, label }: NavItemProps): JSX.Element {
  return (
    <NavLink
      to={to}
      end={to !== '/'}
      className={({ isActive }) =>
        cn(
          'flex items-center gap-3 px-4 py-3 mx-2 rounded-lg',
          'text-text-secondary font-medium',
          'hover:text-text-primary hover:bg-background-tertiary',
          'transition-all duration-150',
          isActive && 'text-navy bg-navy/10 border-l-2 border-navy ml-0 rounded-l-none'
        )
      }
    >
      {icon}
      <span>{label}</span>
    </NavLink>
  );
}

export function Sidebar(): JSX.Element {
  return (
    <aside className="fixed left-0 top-0 bottom-0 w-64 bg-background/95 backdrop-blur-xl border-r border-white/5 flex flex-col z-40">
      {/* Logo + Wordmark */}
      <div className="flex items-center gap-3 px-4 py-5">
        <img
          src="/logo.png"
          alt="Resonance"
          className="h-10 w-10 rounded-xl hover:shadow-[0_0_20px_rgba(90,106,125,0.4)] transition-shadow duration-300"
        />
        <img
          src="/wordmark.png"
          alt="resonance"
          className="h-5 brightness-0 invert opacity-90"
        />
      </div>

      {/* Main Navigation */}
      <nav className="flex flex-col gap-1 py-2">
        <NavItem to="/" icon={<Home size={20} />} label="Home" />
        <NavItem to="/search" icon={<Search size={20} />} label="Search" />
        <NavItem to="/library" icon={<Library size={20} />} label="Your Library" />
      </nav>

      {/* Divider */}
      <div className="mx-4 my-2 h-px bg-white/5" />

      {/* Playlists Section */}
      <div className="flex-1 overflow-hidden">
        <div className="px-6 py-2">
          <span className="text-overline text-text-muted tracking-wider">
            PLAYLISTS
          </span>
        </div>
        <ScrollArea className="h-[calc(100%-40px)]">
          <div className="flex flex-col gap-1 pb-4">
            {/* Placeholder playlists - will be dynamic */}
            <NavLink
              to="/playlist/daily-mix"
              className={({ isActive }) =>
                cn(
                  'px-6 py-2 text-sm text-text-secondary',
                  'hover:text-text-primary',
                  'transition-colors duration-150',
                  isActive && 'text-text-primary'
                )
              }
            >
              Daily Mix
            </NavLink>
            <NavLink
              to="/playlist/chill"
              className={({ isActive }) =>
                cn(
                  'px-6 py-2 text-sm text-text-secondary',
                  'hover:text-text-primary',
                  'transition-colors duration-150',
                  isActive && 'text-text-primary'
                )
              }
            >
              Chill Vibes
            </NavLink>
            <NavLink
              to="/playlist/workout"
              className={({ isActive }) =>
                cn(
                  'px-6 py-2 text-sm text-text-secondary',
                  'hover:text-text-primary',
                  'transition-colors duration-150',
                  isActive && 'text-text-primary'
                )
              }
            >
              Workout
            </NavLink>
          </div>
        </ScrollArea>
      </div>

      {/* Bottom Settings */}
      <div className="border-t border-white/5">
        <NavItem to="/settings" icon={<Settings size={20} />} label="Settings" />
      </div>
    </aside>
  );
}
