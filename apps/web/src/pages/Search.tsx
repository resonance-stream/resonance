export default function Search() {
  return (
    <div className="flex flex-1 flex-col p-6">
      <h1 className="text-3xl font-bold">Search</h1>
      <input
        type="text"
        placeholder="Search for songs, albums, or artists..."
        className="input mt-4 w-full max-w-xl"
      />
    </div>
  )
}
