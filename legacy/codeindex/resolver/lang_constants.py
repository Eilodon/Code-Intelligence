ASSIGNMENT_NODES = {
    "python":     ["assignment", "augmented_assignment"],
    "typescript": ["variable_declarator"],
    "javascript": ["variable_declarator"],
    "java":       ["local_variable_declaration"],
    "rust":       ["let_declaration"],
    "go":         ["short_var_declaration", "var_declaration"],
    # C/C++: defer — templates + qualifiers phức tạp
    # C#/Kotlin/Swift: Tier 1/2 coverage đủ, alias tracking lower priority
}
