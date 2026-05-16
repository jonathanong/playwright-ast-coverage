const Page = () => <div/>;
export default memo(Page);
export function Parent() { return <Page/>; }
