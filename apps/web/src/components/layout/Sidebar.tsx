import { NavLink } from 'react-router-dom';
import { Home, Search, Library, Settings, Shield, ListMusic } from 'lucide-react';
import { cn } from '../../lib/utils';
import { ScrollArea } from '../ui/ScrollArea';
import { Skeleton } from '../ui/Skeleton';
import { useAuthStore } from '../../stores/authStore';
import { useMyPlaylists } from '../../hooks/useLibrary';

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
  const user = useAuthStore((state) => state.user);
  const isAdmin = user?.role === 'admin';
  const { data: playlists, isLoading: playlistsLoading } = useMyPlaylists();

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
            {playlistsLoading ? (
              // Loading skeleton
              <>
                <div className="px-6 py-2">
                  <Skeleton className="h-4 w-24" />
                </div>
                <div className="px-6 py-2">
                  <Skeleton className="h-4 w-32" />
                </div>
                <div className="px-6 py-2">
                  <Skeleton className="h-4 w-20" />
                </div>
              </>
            ) : playlists && playlists.length > 0 ? (
              // Dynamic playlists
              playlists.map((playlist) => (
                <NavLink
                  key={playlist.id}
                  to={`/playlist/${playlist.id}`}
                  className={({ isActive }) =>
                    cn(
                      'px-6 py-2 text-sm text-text-secondary',
                      'hover:text-text-primary',
                      'transition-colors duration-150',
                      isActive && 'text-text-primary'
                    )
                  }
                >
                  {playlist.name}
                </NavLink>
              ))
            ) : (
              // Empty state
              <div className="px-6 py-4 text-sm text-text-muted flex flex-col items-center gap-2">
                <ListMusic size={24} className="opacity-50" />
                <span>No playlists yet</span>
              </div>
            )}
          </div>
        </ScrollArea>
      </div>

      {/* Bottom Settings */}
      <div className="border-t border-white/5">
        {isAdmin && (
          <NavItem to="/admin" icon={<Shield size={20} />} label="Admin" />
        )}
        <NavItem to="/settings" icon={<Settings size={20} />} label="Settings" />
      </div>
    </aside>
  );
}
