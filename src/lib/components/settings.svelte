<script lang="ts">
    import type { Settings } from "$lib/rust-bindings/Settings";
    import { invoke } from "@tauri-apps/api/core";

    const settings = $state({
        pin: false
    });

    $effect(() => {
        if (settings.pin) {
        }
        changeSettings({ pin: settings.pin });
    });

    const changeSettings = async (settings: Partial<Settings>) => {
        console.log("ga", settings);
        await invoke("set_settings", { patch: settings });
    };

    // TODO right now: data driven and all (derive from rust type)
    // TODO right now: get set binding ? don't care (faster?)
</script>

<div class="cont">
    <h2>Settings</h2>
    <label>
        pin
        <input id="pin" type="checkbox" bind:checked={settings.pin} />
    </label>
</div>

<style>
    h2 {
        font-size: 1.5rem;
    }
</style>
