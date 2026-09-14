///<reference types="svelte" />
;;
  interface Props {
    open?: boolean;
    onclose?: () => void;
    class?: string;
    children?: import("svelte").Snippet;
  };function $$render() {



  let { open = false, onclose, class: className, children }: Props = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    if (!dialog) return;
    if (open) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  });

  function handleClose() {
    onclose?.();
  }

  // A click on a `<dialog>` that opened with `showModal()` reports coordinates
  // in the backdrop when it lands outside the dialog's own box. Hit-testing the
  // rect is therefore enough to tell backdrop from content — no stopPropagation
  // wrapper is needed, and the one that used to live here was both redundant
  // and an unlabelled interactive `<div>`.
  function handleBackdropClick(e: MouseEvent) {
    const rect = dialog.getBoundingClientRect();
    const outside =
      e.clientX < rect.left || e.clientX > rect.right ||
      e.clientY < rect.top  || e.clientY > rect.bottom;
    if (outside) dialog.close();
  }
;
async () => {



  { const $$_dialog0 = svelteHTML.createElement("dialog", {       "class":["fid-dialog", className].filter(Boolean).join(" "),"onclose":handleClose,"onclick":handleBackdropClick,});dialog = $$_dialog0;
  ;__sveltets_2_ensureSnippet(children?.());
 }


};
return { props: {} as any as Props, exports: {}, bindings: __sveltets_$$bindings(''), slots: {}, events: {} }}
const Dialog__SvelteComponent_ = __sveltets_2_fn_component($$render());
/*Ωignore_startΩ*/type Dialog__SvelteComponent_ = ReturnType<typeof Dialog__SvelteComponent_>;
/*Ωignore_endΩ*/export default Dialog__SvelteComponent_;