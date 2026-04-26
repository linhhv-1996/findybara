<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { slide } from "svelte/transition";
  import { onMount } from "svelte";

  type CtxData = {
    name: string; 
    ext: string; 
    meta: string;
    color: string; 
    actions: string[]; 
    path: string;
    folder_summary: string;
  };

  let info = $state<CtxData | null>(null);
  let query = $state("");
  let isThinking = $state(false);
  let aiAnswer = $state("");
  let barElement: HTMLDivElement | undefined = $state();
  
  let accent = $derived(info?.color ?? "#a855f7");

  // Theo dõi kích thước để báo lại cho Rust điều chỉnh chiều cao cửa sổ
  $effect(() => {
    if (!barElement) return;
    const observer = new ResizeObserver((entries) => {
      for (let entry of entries) {
        const height = entry.target.getBoundingClientRect().height;
        invoke("set_ui_height", { height: Math.ceil(height) + 8 });
      }
    });
    observer.observe(barElement);
    return () => observer.disconnect();
  });

  onMount(() => {
    listen<{ data: CtxData | null }>("finder-state", (e) => {
      info = e.payload.data;
      aiAnswer = ""; 
      query = "";
    });
  });

  async function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") { 
      if (aiAnswer) aiAnswer = "";
      else invoke("hide_findybara");
    }
    
    if (e.key === "Enter" && query) {
      isThinking = true;
      aiAnswer = "";
      try {
        aiAnswer = await invoke("ask_ai", { 
          query, 
          path: info?.path ?? "", 
          ctxName: info?.name ?? "", 
          folderSummary: info?.folder_summary ?? "" 
        });
      } catch (err) { 
        aiAnswer = "Error: " + err;
      } finally { 
        isThinking = false; 
        query = "";
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="root" style="--accent: {accent}">
  {#if info}
    <div class="micro-bar" bind:this={barElement}>
      
      <div class="context-row">
        <span class="icon">{info.ext === "DIR" ? "📁" : (info.ext === "MULTI" ? "📑" : "📄")}</span>
        <span class="name">{info.name}</span>
        {#if info.meta}
          <span class="meta">{info.meta}</span>
        {/if}
        <div class="spacer"></div>
        <button class="close-btn" onclick={() => invoke("hide_findybara")}>×</button>
      </div>

      <div class="input-row">
        <span class="prompt-icon">✨</span>
        <input 
          bind:value={query} 
          placeholder="Ask AI anything about this selection..." 
          autofocus 
          spellcheck="false"
        />
        
        {#if info.actions && info.actions.length > 0}
          <div class="actions">
            {#each info.actions as action}
              <button class="action-btn">{action}</button>
            {/each}
          </div>
        {/if}
      </div>

      {#if isThinking || aiAnswer}
        <div class="ai-expansion" transition:slide={{ duration: 150 }}>
          {#if isThinking}
            <div class="loading">Thinking...</div>
          {:else}
            <div class="ai-text">{aiAnswer}</div>
          {/if}
        </div>
      {/if}
      
    </div>
  {/if}
</div>

<style>
  :global(html, body) { 
    background: transparent !important; 
    margin: 0; padding: 0; 
    overflow: hidden !important;
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro", sans-serif;
  }
  
  .root { 
    width: 100%; 
    display: flex; 
    justify-content: center; 
    pointer-events: none;
    padding: 4px 1px 0 1px; 
    box-sizing: border-box;
  }

  .micro-bar { 
    width: 100%; 
    background: rgba(28, 28, 30, 0.85);
    backdrop-filter: blur(50px) saturate(200%);
    -webkit-backdrop-filter: blur(50px) saturate(200%);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 16px;
    pointer-events: auto;
    overflow: hidden;
    display: flex; flex-direction: column;
  }

  .context-row {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 12px;
    background: rgba(0, 0, 0, 0.1);
  }
  
  .context-row .icon { font-size: 13px; }
  .context-row .name { 
    font-size: 12px; font-weight: 500; color: #d1d5db; 
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    max-width: 300px;
  }
  .context-row .meta { 
    font-size: 11px; color: #888; font-weight: 400;
    white-space: nowrap;
    border-left: 1px solid rgba(255,255,255,0.1);
    padding-left: 8px;
  }
  .spacer { flex: 1; }

  .close-btn { 
    background: none; border: none; color: #666; 
    font-size: 16px; cursor: pointer; padding: 0; line-height: 1;
  }
  .close-btn:hover { color: #ff5f57; }

  .input-row {
    display: flex; align-items: center; gap: 10px;
    padding: 8px 12px;
    border-top: 1px solid rgba(255,255,255,0.06);
  }

  .prompt-icon {
    font-size: 13px; font-weight: 700; color: var(--accent); opacity: 0.8;
  }

  input { 
    flex: 1; background: none; border: none; color: #fff;
    font-size: 14px; outline: none; padding: 0; min-width: 0;
  }
  input::placeholder { color: #555; }

  .actions {
    display: flex;
    gap: 6px;
    margin-left: 8px;
  }

  .action-btn {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.05);
    color: #e2e8f0;
    font-size: 11px;
    font-weight: 500;
    padding: 5px 12px;
    border-radius: 6px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.2s ease;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.15);
    border-color: var(--accent);
    color: #fff;
  }

  .ai-expansion { border-top: 1px solid rgba(255,255,255,0.06); background: rgba(0,0,0,0.2); }
  .ai-text { 
    padding: 12px; color: #cbd5e1; font-size: 13px; line-height: 1.5;
    -webkit-user-select: text; user-select: text; max-height: 300px; overflow-y: auto;
  }
  .loading { padding: 12px; color: var(--accent); font-size: 12px; }

  ::-webkit-scrollbar { width: 4px; }
  ::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.15); border-radius: 10px; }
</style>
