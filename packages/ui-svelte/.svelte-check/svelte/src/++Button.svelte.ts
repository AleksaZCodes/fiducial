///<reference types="svelte" />
;;
  interface Props {
    variant?: "primary" | "secondary" | "ghost" | "destructive";
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    onclick?: () => void;
    type?: "button" | "submit" | "reset";
    class?: string;
    children?: import("svelte").Snippet;
  };function $$render() {



  let {
    variant = "primary",
    size = "md",
    disabled = false,
    onclick,
    type = "button",
    class: className,
    children,
  }: Props = $props();
;
async () => {

  { svelteHTML.createElement("button", {         type,disabled,onclick,...__sveltets_2_empty({"data-variant":variant}),...__sveltets_2_empty({"data-size":size}),"class":["fid-btn", className].filter(Boolean).join(" "),});
  ;__sveltets_2_ensureSnippet(children?.());
 }


};
return { props: {} as any as Props, exports: {}, bindings: __sveltets_$$bindings(''), slots: {}, events: {} }}
const Button__SvelteComponent_ = __sveltets_2_fn_component($$render());
/*Ωignore_startΩ*/type Button__SvelteComponent_ = ReturnType<typeof Button__SvelteComponent_>;
/*Ωignore_endΩ*/export default Button__SvelteComponent_;