<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { fly, fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { onMount } from "svelte";

  type CtxData = { name: string; ext: string; meta: string; color: string; actions: string[] };
  
  let ctx = $state<string>("none");
  let info = $state<CtxData | null>(null);
  let accent = $derived(info?.color ?? "#888888");

  onMount(() => {
    let unlisten: (() => void) | undefined;
    
    // Lắng nghe sự kiện từ Rust bắn lên
    listen<{ctx: string, data: CtxData | null}>("finder-state", (event) => {
      ctx = event.payload.ctx;
      info = event.payload.data;
    }).then(f => {
      unlisten = f;
    });

    // Cleanup khi component bị huỷ
    return () => {
      if (unlisten) unlisten();
    };
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") invoke("hide_findybara");
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="root" style="--accent:{accent}">
  <div class="bar" class:expanded={info !== null}>
    <div class="shimmer"></div>

    <div class="icon-box" class:lit={info !== null}>
      {#key ctx}
        <div class="icon-inner" in:fade={{duration:160}}>
          {#if ctx === "none"}
            <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="8" cy="8" r="2.5"/><path d="M8 1.5v1.2M8 13.3v1.2M1.5 8h1.2M13.3 8h1.2M3.6 3.6l.85.85M11.55 11.55l.85.85M3.6 12.4l.85-.85M11.55 4.45l.85-.85"/></svg>
          {:else if ctx === "folder"}
            <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M1.5 5A1 1 0 012.5 4h3.086a1 1 0 01.707.293L7.207 5.2A1 1 0 007.914 5.5H13.5a1 1 0 011 1v5.5a1 1 0 01-1 1h-11a1 1 0 01-1-1V5z"/></svg>
          {:else}
            <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="8" cy="8" r="6"/></svg>
          {/if}
        </div>
      {/key}
    </div>

    <div class="content">
      {#if info}
        <div class="file-info" in:fly={{x:-12, duration:300, easing:cubicOut}}>
          <span class="fname">{info.name}<span class="fext">.{info.ext.toLowerCase()}</span></span>
          <span class="fmeta">{info.meta}</span>
        </div>
        <div class="sep" in:fade={{duration:200, delay:100}}></div>
        <div class="actions" in:fly={{x:20, duration:400, delay:150, easing:cubicOut}}>
          {#each info.actions as action}
            <button class="btn" class:danger={action==="Delete"}>{action}</button>
          {/each}
        </div>
      {:else}
        <span class="hint" in:fade>Findybara: Select something in Finder...</span>
      {/if}
    </div>

    <button class="x" onclick={()=>invoke("hide_findybara")}>
      <svg width="8" height="8" viewBox="0 0 8 8" fill="none"><path d="M1 1l6 6M7 1L1 7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
    </button>
  </div>
</div>

<style>
  :global(*) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) { background: transparent !important; overflow: hidden; font-family: -apple-system, sans-serif; }

  .root {
    width: 100vw; 
    height: 100vh;
    display: flex; 
    justify-content: center;
    padding-top: 10px;
  }

  .bar {
    height: 52px; 
    display: flex;
    align-items: center; 
    padding: 0 8px 0 10px;
    background: rgba(22, 22, 28, 0.92);
    backdrop-filter: blur(25px) saturate(170%);
    border: 1px solid rgba(255,255,255,0.12);
    border-radius: 26px;
    width: fit-content; 
    min-width: 60px; 
    max-width: 95vw;
    transition: all 0.45s cubic-bezier(0.19, 1, 0.22, 1);
    position: relative;
  }

  .shimmer {
    position: absolute; top: 0;
    left: 20px; right: 20px; height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    opacity: 0.4;
  }

  .icon-box {
    width: 36px; height: 36px; border-radius: 50%;
    background: rgba(255,255,255,0.04);
    display: flex; align-items: center;
    justify-content: center;
    color: rgba(255,255,255,0.2); transition: all 0.3s;
    flex-shrink: 0;
  }
  .icon-box.lit { 
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent); 
  }

  .content { 
    display: flex; 
    align-items: center; 
    overflow: hidden; 
    padding: 0 8px;
    height: 100%;
  }

  .file-info { display: flex; flex-direction: column; gap: 1px; flex-shrink: 0; }
  .fname { color: #fff; font-size: 13px; font-weight: 600; white-space: nowrap; }
  .fmeta { font-family: ui-monospace, monospace; font-size: 9px; color: rgba(255,255,255,0.35); }

  .sep { width: 1px; height: 18px; background: rgba(255,255,255,0.1); margin: 0 12px; flex-shrink: 0; }

  .actions { 
    display: flex; 
    gap: 6px; 
    align-items: center;
    overflow-x: auto;
    scrollbar-width: none;
    padding: 4px 0;
  }
  .actions::-webkit-scrollbar { display: none; }

  .btn {
    height: 28px; 
    padding: 0 12px; 
    border-radius: 14px;
    background: rgba(255,255,255,0.06);
    border: 1px solid rgba(255,255,255,0.08);
    color: rgba(255,255,255,0.6); 
    font-size: 11px; 
    cursor: pointer; 
    transition: all 0.2s;
    display: flex; 
    align-items: center;
    justify-content: center;
    white-space: nowrap; 
    line-height: 1;
  }
  .btn:hover { 
    background: var(--accent); 
    color: #000; 
    transform: translateY(-2px);
  }
  .btn.danger { color: #f87171; }

  /* Bổ sung .hint để ép text trên 1 dòng, không làm phình chiều cao bar */
  .hint {
    color: rgba(255,255,255,0.4);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap; 
    padding: 0 4px;
  }

  .x {
    width: 26px; height: 26px; border-radius: 50%; border: none; background: transparent;
    color: rgba(255,255,255,0.1);
    cursor: pointer; display: flex; align-items: center; justify-content: center;
    flex-shrink: 0;
  }
  .x:hover { background: rgba(255,255,255,0.1); color: #fff; }
</style>
