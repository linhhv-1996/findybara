<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { onMount } from "svelte";

  type CtxData = {
    name: string;
    ext: string;
    meta: string;
    color: string;
    actions: string[]; // Tạm thời không dùng đến
  };

  let ctx = $state<"none" | "file" | "folder" | "multi">("none");
  let info = $state<CtxData | null>(null);
  let accent = $derived(info?.color ?? "#888888");

  // Tên hiển thị ngắn gọn
let displayTitle = $derived.by(() => {
    if (!info) return "";
    
    // Nếu là chọn nhiều, backend đã trả về sẵn "X items selected"
    if (ctx === "multi") return info.name; 
    
    // Nếu là folder (dù là đang đứng trong đó hay được chọn), hiện tên folder ra
    if (ctx === "folder") return info.name; 
    
    // Nếu là file, vẫn giấu tên thật (nếu bạn vẫn muốn giữ rule cũ)
    // Hoặc bạn có thể đổi thành `return info.name;` để hiện luôn tên file
    if (ctx === "file") return "1 file selected"; 
    
    return info.name;
  });

  onMount(() => {
    let unlisten: (() => void) | undefined;
    listen<{ ctx: string; data: CtxData | null }>("finder-state", (event) => {
      ctx = event.payload.ctx as any;
      info = event.payload.data;
    }).then((f) => (unlisten = f));

    return () => unlisten && unlisten();
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      invoke("hide_findybara");
    }
  }

  function getContextIcon() {
    if (ctx === "folder") {
      return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>`;
    }
    if (ctx === "multi") {
      return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 9h6"/><path d="M9 15h6"/></svg>`;
    }
    return `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>`;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="root" style="--accent: {accent}">
  <div class="bar" class:has-info={!!info}>
    <div class="shimmer"></div>

    <div class="main-row">
      <div class="icon-box" class:lit={!!info}>
        {@html getContextIcon()}
      </div>

      <div class="content">
        {#if info}
          <div class="file-info" in:fly={{ x: -8, duration: 220, easing: cubicOut }}>
            <span class="title">{displayTitle}</span>
            <span class="dot">•</span>
            <span class="ext-badge">{info.ext}</span>
            <span class="meta">{info.meta}</span>
          </div>
        {:else}
          <div class="hint">Findybara • Chọn item trong Finder</div>
        {/if}
      </div>

      <button class="close-btn" onclick={() => invoke("hide_findybara")}>✕</button>
    </div>
  </div>
</div>

<style>
  :global(body) {
    background: transparent !important;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, sans-serif;
    margin: 0;
    padding: 0;
  }

  .root {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center; /* Giữ thanh này ở chính giữa cửa sổ Tauri */
    pointer-events: none;
  }

  .bar {
    pointer-events: auto;
    background: rgba(28, 28, 35, 0.95);
    backdrop-filter: blur(32px) saturate(180%);
    border: 1px solid rgba(255,255,255,0.13);
    border-radius: 999px; /* Tròn hẳn hai đầu kiểu viên thuốc */
    overflow: hidden;
    position: relative;
    padding: 0 6px;
    height: 36px;
    /* min-width: 350px; */
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .shimmer {
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 1px;
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    opacity: 0.4;
    animation: shimmer 3s linear infinite;
  }
  @keyframes shimmer {
    from { transform: translateX(-150%); }
    to { transform: translateX(300%); }
  }

  .main-row {
    display: flex;
    align-items: center;
    height: 100%;
    gap: 8px;
  }

  .icon-box {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: rgba(255,255,255,0.08);
    display: flex;
    align-items: center;
    justify-content: center;
    color: #aaa;
    flex-shrink: 0;
  }
  .icon-box.lit {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 30%, transparent);
  }

  .content {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
  }

  /* Dàn ngang toàn bộ text */
  .file-info {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .title {
    font-size: 13px;
    font-weight: 500;
    color: #fff;
    white-space: nowrap;

    white-space: nowrap;       /* Cấm xuống dòng */
    overflow: hidden;          /* Giấu phần text bị tràn */
    text-overflow: ellipsis;   /* Biến phần tràn thành dấu ... */
    flex-shrink: 1;
  }

  .dot {
    color: rgba(255,255,255,0.3);
    font-size: 10px;
  }

  .ext-badge {
    background: rgba(255,255,255,0.15);
    padding: 2px 6px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 600;
    color: #eee;
    letter-spacing: 0.2px;
  }

  .meta {
    font-size: 12px;
    color: rgba(255,255,255,0.6);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .hint {
    color: rgba(255,255,255,0.48);
    font-size: 12.5px;
  }

  .close-btn {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: transparent;
    border: none;
    color: rgba(255,255,255,0.4);
    font-size: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .close-btn:hover {
    background: rgba(255,255,255,0.12);
    color: #fff;
  }
</style>
