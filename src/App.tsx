import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="container mx-auto px-4 pt-10 flex flex-col items-center text-center">
      <h1 className="text-4xl font-bold mb-8">Welcome to Tauri + React</h1>

      <div className="flex justify-center gap-8 mb-6">
        <a href="https://vite.dev" target="_blank" className="transition-all hover:drop-shadow-[0_0_2em_#747bff]">
          <img src="/vite.svg" className="h-24 p-6" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank" className="transition-all hover:drop-shadow-[0_0_2em_#24c8db]">
          <img src="/tauri.svg" className="h-24 p-6" alt="Tauri logo" />
        </a>
        <a href="https://react.dev" target="_blank" className="transition-all hover:drop-shadow-[0_0_2em_#61dafb]">
          <img src={reactLogo} className="h-24 p-6" alt="React logo" />
        </a>
      </div>
      <p className="mb-6 text-muted-foreground">Click on the Tauri, Vite, and React logos to learn more.</p>

      <form
        className="flex gap-2 mb-4"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
          className="px-4 py-2 border border-input rounded-lg bg-background focus:outline-none focus:ring-2 focus:ring-ring"
        />
        <Button type="submit">Greet</Button>
      </form>
      <p className="text-lg font-medium">{greetMsg}</p>
    </main>
  );
}

export default App;
