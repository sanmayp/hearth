<script lang="ts">
  import { page } from '$app/state';
</script>

<main>
  <img class="state" src="/art/error-state.webp" alt="" width="768" height="768" />
  <h1>Wrong turn.</h1>
  <p>Hearth has no page here ({page.status}).</p>
  <a href="/">Back to Today</a>
</main>

<style>
  main {
    max-width: 40rem;
    margin: 0 auto;
    padding: 2rem 1.25rem 4rem;
  }
  h1 {
    font-size: var(--text-display);
    margin: 0;
  }
  .state {
    display: block;
    width: 8rem;
    height: auto;
    border-radius: 1rem;
    margin-bottom: 1rem;
  }
  a {
    color: var(--accent);
  }
</style>
