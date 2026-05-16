import React from 'react';
const Nav = () => (
  <nav>
    <a href="/communities">Communities</a>
    <a href={`/communities/${slug}`}>Community</a>
    <a href="/api/v1/users">Users API</a>
  </nav>
);
export default Nav;
