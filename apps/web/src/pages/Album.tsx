import { useParams } from 'react-router-dom'

export default function Album() {
  const { id } = useParams<{ id: string }>()

  return (
    <div className="flex flex-1 flex-col p-6">
      <h1 className="text-3xl font-bold">Album</h1>
      <p className="mt-2 text-text-secondary">Album ID: {id}</p>
    </div>
  )
}
