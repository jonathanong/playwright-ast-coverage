import { PageWithAside } from '@/components/PageWithAside';

export default function SettingsPage() {
  return (
    <PageWithAside
      aside={
        <nav>
          <a href="/settings/profile">Profile</a>
          <a href="/settings/security">Security</a>
        </nav>
      }
    >
      <h1>Settings</h1>
    </PageWithAside>
  );
}
