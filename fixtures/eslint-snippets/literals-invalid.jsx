function A({ testId }) {
  page.getByTestId(id);
  page.getByTestId();
  return (
    <>
      <button data-pw />
      <button data-pw={id} />
      <button data-pw={testId} />
      <button data-pw={`user-${id}`} />
      <button data-other={id} />
    </>
  );
}
