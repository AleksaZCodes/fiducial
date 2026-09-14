///<reference types="svelte" />
;;
  interface Props {
    class?: string;
    children?: import("svelte").Snippet;
  };function $$render() {


  let { class: className, children }: Props = $props();
;
async () => {

 { svelteHTML.createElement("div", { "class":["fid-card", className].filter(Boolean).join(" "),});
  ;__sveltets_2_ensureSnippet(children?.());
 }


};
return { props: {} as any as Props, exports: {}, bindings: __sveltets_$$bindings(''), slots: {}, events: {} }}
const Card__SvelteComponent_ = __sveltets_2_fn_component($$render());
/*Ωignore_startΩ*/type Card__SvelteComponent_ = ReturnType<typeof Card__SvelteComponent_>;
/*Ωignore_endΩ*/export default Card__SvelteComponent_;