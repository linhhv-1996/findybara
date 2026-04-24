<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { fade, slide } from "svelte/transition";
  import { onMount } from "svelte";

  type CtxData = {
    name: string; ext: string; meta: string;
    color: string; actions: string[]; path: string;
    folder_summary: string;
  };

  let info = $state<CtxData | null>(null);
  let query = $state("");
  let isThinking = $state(false);
  let aiAnswer = $state("");
  
  let barElement: HTMLDivElement | undefined = $state();

  // Dùng màu accent làm điểm nhấn, nếu không có thì dùng màu tím hiện đại
  let accent = $derived(info?.color ?? "#a855f7");
  let filteredActions = $derived.by(() => {
    if (!info || !query) return info?.actions ?? [];
    return info.actions.filter(a => a.toLowerCase().includes(query.toLowerCase()));
  });

  $effect(() => {
    if (!barElement) return;
    const observer = new ResizeObserver((entries) => {
      for (let entry of entries) {
        const actualHeight = entry.target.getBoundingClientRect().height;
        invoke("set_ui_height", { height: Math.ceil(actualHeight + 24) });
      }
    });
    observer.observe(barElement);
    return () => observer.disconnect();
  });

  onMount(() => {
    listen<{ data: CtxData | null }>("finder-state", (e) => {
      info = e.payload.data;
      aiAnswer = ""; query = "";
    });
  });

  async function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") { invoke("hide_findybara"); aiAnswer = ""; }
    if (e.key === "Enter" && query) {
      if (filteredActions.length > 0) {
        invoke("run_action", { action: filteredActions[0] });
        query = "";
      } else {
        isThinking = true; aiAnswer = "";
        try {
          aiAnswer = await invoke("ask_ai", { 
            query, path: info?.path ?? "", ctxName: info?.name ?? "", folderSummary: info?.folder_summary ?? "" 
          });
        } catch (err) { aiAnswer = "Error: " + err; }
        finally { isThinking = false; query = ""; }
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="root" style="--accent: {accent}">
  {#if info}
    <div class="command-bar" bind:this={barElement} transition:slide={{ duration: 250, easing: (t) => t * (2 - t) }}>
      
      <div class="context-header">
        <div class="context-badge">
          <span class="icon">{info.ext === "DIR" ? "📂" : (info.ext === "MULTI" ? "📦" : "📄")}</span>
          <span class="name">{info.name}</span>
          {#if info.meta}
            <span class="divider">/</span>
            <span class="meta">{info.meta}</span>
          {/if}
        </div>
        <button class="close-btn" onclick={() => { invoke("hide_findybara"); aiAnswer = ""; }} title="Close (Esc)">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
        </button>
      </div>

      <div class="input-row">
        <div class="prompt-icon">✨</div>
        <input 
          bind:value={query} 
          placeholder="Ask anything or type a command..." 
          autofocus 
          spellcheck="false"
        />
        
        {#if query && filteredActions.length > 0}
          <div class="action-hint" in:fade={{duration: 150}}>
            <span>{filteredActions[0]}</span>
            <kbd>↵</kbd>
          </div>
        {/if}
      </div>

      {#if isThinking || aiAnswer}
        <div class="ai-wrapper" transition:slide={{ duration: 300 }}>
          <div class="ai-separator"></div>
          <div class="ai-section">
            {#if isThinking}
              <div class="loading-dots"><span></span><span></span><span></span></div>
            {:else}
              <div class="ai-content">{aiAnswer}</div>
            {/if}
          </div>
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
    overscroll-behavior: none; 
    -webkit-user-select: none; 
    user-select: none;
    font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI", Roboto, Helvetica, Arial, sans-serif; 
  }

  input, .ai-content {
    -webkit-user-select: text; 
    user-select: text;
  }
  
  .root { 
    width: 100%; height: 100%; 
    display: flex; justify-content: center; align-items: flex-start; 
    pointer-events: none; 
  }

  /* KHUNG MAIN: GLASSMORPHISM CHUẨN MAC */
  .command-bar { 
    width: 540px;
    margin-top: 5px;
    background: rgba(28, 28, 33, 0.95); /* Trong suốt hơn */
    backdrop-filter: blur(40px) saturate(180%);
    -webkit-backdrop-filter: blur(40px) saturate(180%);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 16px;
    pointer-events: auto;
    overflow: hidden;
    display: flex; flex-direction: column;
  }

  /* --- TẦNG 1: HEADER --- */
  .context-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 16px 2px 16px;
  }

  .context-badge {
    display: flex; align-items: center; gap: 6px;
    background: color-mix(in srgb, var(--accent) 15%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent);
    padding: 4px 10px;
    border-radius: 8px;
    font-size: 12px;
  }

  .context-badge .icon { font-size: 12px; }
  .context-badge .name { 
    font-weight: 600; color: #f1f5f9;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis; 
    max-width: 200px;
  }
  .context-badge .divider { color: rgba(255,255,255,0.2); font-weight: 300; }
  .context-badge .meta { color: #94a3b8; font-weight: 500; }

  .close-btn {
    background: rgba(255, 255, 255, 0.05); 
    border: 1px solid rgba(255, 255, 255, 0.05); 
    color: #94a3b8; 
    cursor: pointer; 
    width: 24px; height: 24px; border-radius: 6px;
    display: flex; align-items: center; justify-content: center;
    transition: all 0.2s ease;
  }
  .close-btn:hover { background: rgba(255, 255, 255, 0.1); color: white; transform: scale(1.05); }

  /* --- TẦNG 2: INPUT --- */
  .input-row {
    display: flex; align-items: center;
    padding: 12px 16px 16px 16px;
    gap: 12px;
  }

  .prompt-icon { 
    font-size: 15px; 
    background: linear-gradient(135deg, #a855f7, #ec4899);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    filter: drop-shadow(0 2px 4px rgba(168, 85, 247, 0.4));
  }

  input { 
    flex: 1; background: none; border: none; color: white; 
    font-size: 13px; /* Chữ to gõ sướng hơn */
    font-weight: 400; outline: none; min-width: 0;
  }
  input::placeholder { color: #64748b; font-weight: 400; }

  /* Nút Action Hint chuẩn UI mới */
  .action-hint {
    background: linear-gradient(180deg, rgba(255,255,255,0.1), rgba(255,255,255,0.05));
    border: 1px solid rgba(255,255,255,0.1);
    padding: 6px 10px; 
    border-radius: 8px;
    font-size: 12px; font-weight: 500; color: #f8fafc;
    display: flex; align-items: center; gap: 8px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.2);
  }
  .action-hint kbd { 
    background: rgba(0,0,0,0.4); border: 1px solid rgba(255,255,255,0.1);
    padding: 2px 6px; border-radius: 4px; font-family: monospace; font-size: 11px; color: #cbd5e1;
  }

  /* --- TẦNG 3: AI SECTION --- */
  .ai-separator {
    height: 1px; width: 100%;
    background: linear-gradient(90deg, transparent, rgba(255,255,255,0.08), transparent);
  }

  .ai-section {
    padding: 16px; 
    background: rgba(0, 0, 0, 0.15); /* Làm tối nền AI một chút cho nổi chữ */
    max-height: 150px; overflow-y: auto;
  }

  .ai-content { 
    color: #cbd5e1; font-size: 13px; line-height: 1.6; 
    white-space: pre-wrap; font-weight: 400;
  }

  /* Loading ngầu hơn */
  .loading-dots { display: flex; gap: 6px; padding: 4px 0; align-items: center; height: 22px; }
  .loading-dots span { 
    width: 6px; height: 6px; background: #a855f7; border-radius: 50%; 
    animation: bounce 1s infinite alternate;
    box-shadow: 0 0 8px rgba(168, 85, 247, 0.6);
  }
  .loading-dots span:nth-child(2) { animation-delay: 0.2s; }
  .loading-dots span:nth-child(3) { animation-delay: 0.4s; }

  @keyframes bounce { 
    from { transform: translateY(0); opacity: 0.6; } 
    to { transform: translateY(-4px); opacity: 1; } 
  }
  
  /* Thanh cuộn tàng hình thanh lịch */
  ::-webkit-scrollbar { width: 4px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.2); border-radius: 10px; }
  ::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.3); }
</style>
