import dynamic from 'next/dynamic'

// bad: wraps a server component
const BadComp = dynamic(() => import('./ServerComp'))

// good: wraps a client component
const GoodComp = dynamic(() => import('./ClientComp'))
