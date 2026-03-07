import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Route, HashRouter as Router, Routes } from "react-router-dom";
import { Auth } from "./pages/Auth";
import { Success } from "./pages/Success";

function App() {
  const [isLogged, setIsLogged] = useState(false);
  const [version, setVersion] = useState("");

  useEffect(() => {
    invoke("get_version").then((v) => setVersion(v));
  }, []);

  return (
    <Router>
      <Routes>
        <Route path="/" element={<Auth setIsLogged={setIsLogged} />} />
        <Route
          path="/success"
          element={<Success isLogged={isLogged} setIsLogged={setIsLogged} />}
        />
      </Routes>
      {version && (
        <span className="fixed bottom-2 right-3 text-xs text-neutral-400 dark:text-neutral-600">
          {version}
        </span>
      )}
    </Router>
  );
}

export default App;
