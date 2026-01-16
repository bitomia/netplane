import "./App.css";
import { HashRouter as Router, Routes, Route } from 'react-router-dom';
import { useState } from "react";
import { Auth } from './pages/auth';
import { Success } from './pages/success';

function App() {

  const [isLogged, setisLogged] = useState(false);

  return (
    <Router>
      <Routes>
        <Route path="/" element={<Auth setisLogged={setisLogged}/>}/>
        <Route path="/success" element={<Success isLogged={isLogged}/>}/>
      </Routes>
    </Router>
  )
}

export default App;
