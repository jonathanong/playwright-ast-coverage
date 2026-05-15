function A({ testId = "save", other = "open" }) {
  page.getByTestId(other);
  return (
    <>
      <button data-pw="save" />
      <button data-testid={"publish"} />
      <button data-pw={testId} />
      <button data-pw={`user-${id}`} />
      <button data-pw={`static`} />
      <button data-pw={`${id}-user`} />
    </>
  );
}
