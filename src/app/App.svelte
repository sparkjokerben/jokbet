<script lang="ts">
  // Routes the on-demand windows by ?view= (stats / settings / onboarding).
  const view = new URLSearchParams(location.search).get("view") ?? "stats";
</script>

<main>
  <p>{view}</p>
</main>
