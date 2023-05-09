import "./App.css";

function App() {
  return (
    <div className="App">
      <header>
        <h1>place.dog</h1>
      </header>
      <section className="main">
        <h2>
          A simple service to get cute dogs as placeholders for your websites
          and designs. Just add a width and height to the end of the url.
        </h2>
        <img src="https://place.dog/300/200" alt="Dog" />
        <p>Try in your Browser</p>
        <code>https://place.dog/300/200</code>
        <p>Or, in your Terminal</p>
        <code>curl -i https://place.dog/300/200 -o doggo.jpeg</code>

        <div className="submit-prompt">
          <p>
            Got a cute doggo? Submit yours to{" "}
            <a href="https://twitter.com/joshghent">@joshghent</a> on Twitter
          </p>
        </div>
      </section>
      <footer>
        <p>
          Built by <a href="https://joshghent.com">Josh Ghent</a> at{" "}
          <a href="https://loginllama.app">
            LoginLlama - Suspicious Login Detection powered by AI.
          </a>
        </p>
      </footer>
    </div>
  );
}

export default App;
