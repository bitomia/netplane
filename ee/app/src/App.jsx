import "./App.css";
import { HashRouter as Router, Routes, Route } from 'react-router-dom';
import { useState } from "react";
import { Auth } from './pages/Auth';
import { Success } from './pages/Success';

function App() {
  const [isLogged, setIsLogged] = useState(false);

  return (
    <Router>
      <Routes>
        <Route path="/" element={<Auth setIsLogged={setIsLogged}/>}/>
        <Route path="/success" element={<Success isLogged={isLogged} setIsLogged={setIsLogged}/>}/>
      </Routes>
    </Router>
  )
}

export default App;
