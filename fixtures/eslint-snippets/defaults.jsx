function Good({ testId = "save", nested: { inner = "ok" } = {} }) {
  return (
    <>
      <button data-pw={testId} />
      <button data-pw={inner} />
      <button data-testid:foo="ignored" />
    </>
  );
}

function Bad({ testId, other = id, ...rest }) {
  return (
    <>
      <button data-pw={testId} />
      <button data-pw={other} />
    </>
  );
}
