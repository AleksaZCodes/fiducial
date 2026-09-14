///<reference types="svelte" />
;;
  interface Props {
    variant?: "default" | "secondary" | "destructive" | "outline";
    class?: string;
    children?: import("svelte").Snippet;
  };function $$render() {


  let { variant = "default", class: className, children }: Props = $props();
;
async () => {

  { svelteHTML.createElement("span", {    ...__sveltets_2_empty({"data-variant":variant}),"class":["fid-badge", className].filter(Boolean).join(" "),});
  ;__sveltets_2_ensureSnippet(children?.());
 }


};
return { props: {} as any as Props, exports: {}, bindings: __sveltets_$$bindings(''), slots: {}, events: {} }}
const Badge__SvelteComponent_ = __sveltets_2_fn_component($$render());
/*Ωignore_startΩ*/type Badge__SvelteComponent_ = ReturnType<typeof Badge__SvelteComponent_>;
/*Ωignore_endΩ*/export default Badge__SvelteComponent_;