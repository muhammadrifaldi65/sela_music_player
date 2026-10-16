// Run in the Vite preview console with a selected track:
// await (await import('/tests/ui-smoke.js')).checkLyricsExit()
export async function checkLyricsExit() {
  const assert = (value, message) => { if (!value) throw new Error(message); };
  const frame = () => new Promise(requestAnimationFrame);
  const settle = async () => {
    for (let i = 0; i < 120; i++) {
      await frame();
      const panel = document.querySelector('.expanded-player');
      if (!panel || panel.classList.contains('is-open')) return;
    }
    throw new Error('Player transition did not settle');
  };
  assert(!document.querySelector('.expanded-player'), 'Close lyrics before running');
  const trigger = document.querySelector('[aria-label="Buka artwork dan lirik"], [aria-label="Open artwork and lyrics"]');
  assert(trigger, 'Player trigger missing');
  trigger.focus();
  trigger.click();
  await settle();
  assert(document.querySelector('.expanded-player.is-open'), 'Select a track first');
  document.querySelector('.back-player').click();
  await frame();
  if (!matchMedia('(prefers-reduced-motion: reduce)').matches && document.documentElement.dataset.animations !== 'false') {
    assert(document.querySelector('.expanded-player.is-closing'), 'Exit must remain mounted during motion');
    assert(document.querySelector('.shell').inert, 'Background must stay inert during exit');
  }
  await settle();
  await frame();
  assert(!document.querySelector('.expanded-player'), 'Exit must remove player');
  assert(!document.querySelector('.shell').inert, 'Background must unlock');
  assert(document.activeElement === trigger, 'Focus must return to trigger');
  assert(document.documentElement.scrollWidth <= innerWidth, 'Page must not overflow horizontally');
  return 'Lyrics exit, focus restoration, and viewport checks passed';
}
