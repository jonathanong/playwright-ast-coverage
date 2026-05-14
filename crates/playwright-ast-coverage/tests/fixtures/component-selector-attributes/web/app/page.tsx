function SaveButton({ dataPw }: { dataPw: string }) {
  return <button data-pw={dataPw}>Save</button>;
}

export default function Page() {
  return <SaveButton dataPw="save" />;
}
