import { useState } from 'react'
import './App.css'

function App() {
  const [count, setCount] = useState(0)

  return (
    <div className="flex flex-col items-center justify-center min-h-screen bg-gray-900 text-white">
      <h1 className="text-4xl font-bold mb-8">voiceTypeless</h1>
      <p className="text-gray-400 mb-6">Local-first voice dictation</p>
      <div className="p-6">
        <button
          className="px-6 py-3 bg-blue-600 hover:bg-blue-700 rounded-lg text-lg transition-colors"
          onClick={() => setCount((count) => count + 1)}
        >
          count is {count}
        </button>
      </div>
    </div>
  )
}

export default App
