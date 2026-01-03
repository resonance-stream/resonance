import { useNavigate } from 'react-router-dom';
import { ChevronLeft, ChevronRight, LogOut } from 'lucide-react';
import { Button } from '../ui/Button';
import { SyncStatusIndicator } from '../sync/SyncStatusIndicator';
import { useAuthStore } from '../../stores/authStore';

export function Header(): JSX.Element {
  const navigate = useNavigate();
  const { user, logout } = useAuthStore();

  const handleLogout = (): void => {
    logout();
    navigate('/login');
  };

  return (
    <header className="sticky top-0 z-30 flex items-center justify-between gap-4 px-6 py-4 bg-background/80 backdrop-blur-lg border-b border-white/5">
      {/* Navigation Arrows */}
      <div className="flex items-center gap-2">
        <Button
          variant="icon"
          size="icon"
          onClick={() => navigate(-1)}
          aria-label="Go back"
        >
          <ChevronLeft size={20} />
        </Button>
        <Button
          variant="icon"
          size="icon"
          onClick={() => navigate(1)}
          aria-label="Go forward"
        >
          <ChevronRight size={20} />
        </Button>
      </div>

      {/* User Section */}
      <div className="flex items-center gap-4">
        <SyncStatusIndicator />
        {user && (
          <>
            <span className="text-sm text-text-secondary">
              {user.displayName || user.email}
            </span>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleLogout}
              className="gap-2"
            >
              <LogOut size={16} />
              <span>Logout</span>
            </Button>
          </>
        )}
      </div>
    </header>
  );
}
