describe("Workspace Directory", () => {
  // Currently only works against Prod or if you manually create a Workspace named "Selenium Testing" in your local/dev cluster environment
  it("Shows title and loads directory items", () => {
    cy.visit(`/workspaces/directory`);
    cy.contains("h1", "My workspaces");
    cy.contains("h4", Cypress.env("TEST_WORKSPACE_NAME"));
  });

  it("Navigates to workspace page", () => {
    cy.visit(`/workspaces/directory`);

    cy.get("h4")
      .contains(Cypress.env("TEST_WORKSPACE_NAME"))
      .click()
      .then(($h3) => {
        const text = $h3.text();
        cy.contains("h1", text);
        cy.contains("h2", "Most recent items");
      });
  });
});
