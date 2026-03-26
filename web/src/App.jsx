import { BrowserRouter, Routes, Route } from 'react-router-dom';
import RequestsPage from "./pages/RequestsPage.jsx";
import LoginPage from "./pages/LoginPage.jsx";

export default function App() {
  return (
      <BrowserRouter>
          <Routes>
              <Route path="/" element={<RequestsPage />} />
              <Route path="/login" element={<LoginPage />} />
          </Routes>
      </BrowserRouter>
  )
}
