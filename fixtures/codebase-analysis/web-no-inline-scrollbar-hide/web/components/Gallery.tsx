export default function Gallery({ images }: Props) {
  return (
    <div className="overflow-auto scrollbar-hide flex gap-4">
      {images.map((img) => (
        <img key={img.id} src={img.src} alt={img.alt} />
      ))}
    </div>
  );
}
