import './App.css';

function App() {
  return (
    <div className="App">
      <header>
        <h1>place.dogs</h1>
      </header>
      <section className="main">
        <p>A simple service to get cute dogs as placeholders for your websites and designs. Just add a width and height to the end of the url.</p>
        <img src="https://dog.ghent.cloud/300/200" alt="Dog" />
        <p>Try in your Browser</p>
        <code>https://place.dog/300/200</code>
        <p>Or, in your Terminal</p>
        <code>curl -i https://place.dog/300/200 -o doggo.jpeg</code>
      </section>
      <footer>
        <p>Built by <a href="https://joshghent.com">Josh Ghent</a> at <a href="https://turboapi.dev">turboapi.dev</a></p>
      </footer>
    </div>
  );
}

export default App;
