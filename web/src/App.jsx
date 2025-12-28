import { BrowserRouter, Routes, Route } from 'react-router-dom';
import RequestsPage from "./pages/RequestsPage.jsx";

export default function App() {
  return (
      <BrowserRouter>
          <Routes>
              <Route path="/" element={<RequestsPage />} />
          </Routes>
      </BrowserRouter>
  )
}