import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { TopographicBackground } from '../ui/TopographicBackground';

interface MainLayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: MainLayoutProps): JSX.Element {
  return (
    <div className="flex min-h-screen">
      <TopographicBackground />

      <Sidebar />
      <div className="flex flex-1 flex-col ml-64">
        <Header />
        <main className="flex-1 overflow-auto pb-24">
          {children}
        </main>
      </div>
    </div>
  );
}
