<script lang="ts">
    import Settings from "$lib/components/settings.svelte";
    import { listen } from "@tauri-apps/api/event";
    import { onMount } from "svelte";

    let link = $state("");

    const performSurgery = async (e: Event) => {
        e.preventDefault();
    };

    // TODO settings: settings proper
    // TODO settings: debug settings category
    let requiresReopen = $state(false);
    onMount(async () => {
        await listen<boolean>("requires_reopen", e => {
            requiresReopen = e.payload;
        });
    });
</script>

<main>
    {#if requiresReopen}
        <p>warn: you need to reopen the app</p>
    {/if}
    <h1>
        They Did Surgery
        <br />
        On A Link
    </h1>

    <form onsubmit={performSurgery}>
        <input type="text" id="link-input" placeholder="Bring the link..." bind:value={link} />
        <button type="submit">Perform surgery</button>
    </form>
    <Settings />
</main>

<style>
    main {
        height: 100%;

        background-color: var(--bg-color);
        border-radius: 16px;

        & button {
            all: unset;
            cursor: pointer;

            background: var(--accent-color);
            border: var(--ui-border-size) solid var(--text-color);
            padding: var(--ui-padding);
        }
    }
</style>
