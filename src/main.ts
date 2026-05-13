import { createApp, reactive } from "vue";
import App from "./App.vue";

const app = createApp(App);

type ToastType = 'success' | 'error' | 'info';

interface ToastState {
  message: string;
  type: ToastType;
  visible: boolean;
}

const toastState = reactive<ToastState>({
  message: '',
  type: 'info',
  visible: false
});

let toastTimeout: ReturnType<typeof setTimeout> | null = null;

function showToast(message: string, type: ToastType = 'info') {
  if (toastTimeout) clearTimeout(toastTimeout);
  toastState.message = message;
  toastState.type = type;
  toastState.visible = true;
  toastTimeout = setTimeout(() => {
    toastState.visible = false;
  }, 3000);
}

function hideToast() {
  toastState.visible = false;
}

app.config.globalProperties.$toast = { show: showToast, hide: hideToast };

app.mount("#app");

export { toastState, showToast, hideToast };
