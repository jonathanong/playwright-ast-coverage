// allowlisted: sidebar is permitted to use useIsMobile
import { useIsMobile } from '@/hooks/use-mobile'

export const Sidebar = () => {
  const isMobile = useIsMobile()
  return <aside>{isMobile ? 'mobile' : 'desktop'}</aside>
}
