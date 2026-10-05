import { createRoot } from 'react-dom/client'
import App from './App'
import 'pixel-retroui/dist/index.css'
import 'pixel-retroui/dist/fonts.css'
import './styles/base.css'

createRoot(document.getElementById('root')!).render(<App />)
