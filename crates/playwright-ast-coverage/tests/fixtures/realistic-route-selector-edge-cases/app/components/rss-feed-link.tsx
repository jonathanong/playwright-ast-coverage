type RssFeedLinkProps = {
  href: string;
  "data-pw"?: string;
};

export function RssFeedLink({
  href,
  "data-pw": dataPw = "rss-feed-link",
}: RssFeedLinkProps) {
  return (
    <a href={href} data-pw={dataPw}>
      RSS
    </a>
  );
}

