function registerPageTools() {
  const form = document.querySelector("form");

  if (form) {
    form.setAttribute("toolname", "replace_me");
    form.setAttribute("tooldescription", "Replace with the user-facing action");
    for (const control of form.elements) {
      if (control.name && !control.hasAttribute("toolparamdescription")) {
        control.setAttribute("toolparamdescription", `Replace with a description for ${control.name}`);
      }
    }
  }
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", registerPageTools, { once: true });
} else {
  registerPageTools();
}
