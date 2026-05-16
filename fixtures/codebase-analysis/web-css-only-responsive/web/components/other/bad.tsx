import { useIsMobile } from '@/hooks/use-mobile'

export const Nav = () => {
  const isMobile = useIsMobile()
  return isMobile ? <div>mobile</div> : <div>desktop</div>
}
