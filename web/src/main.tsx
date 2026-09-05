import { render } from 'preact';
import { App } from './App';
import './styles.css';

render(<App />, document.getElementById('app') as HTMLElement);

if ('serviceWorker' in navigator && !import.meta.env.DEV) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('./sw.js').catch(() => {
      // 離線快取不可用時不影響主要功能
    });
  });
}
