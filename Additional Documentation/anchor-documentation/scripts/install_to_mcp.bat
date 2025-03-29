@echo off
REM Script to install Anchor documentation to the solana-hft-knowledge MCP server

REM Define paths
set MCP_SERVER_DIR=C:\Users\tonal\AppData\Roaming\Roo-Code\MCP\solana-hft-knowledge
set DOCS_DIR=%MCP_SERVER_DIR%\docs\anchor
set SRC_DIR=%MCP_SERVER_DIR%\src
set ANCHOR_DOCS_DIR=%CD%\simple-pages

echo Installing Anchor documentation to solana-hft-knowledge MCP server...

REM Create docs directory if it doesn't exist
echo Creating documentation directory...
if not exist "%DOCS_DIR%" mkdir "%DOCS_DIR%"

REM Copy documentation files
echo Copying documentation files...
xcopy /E /I /Y "%ANCHOR_DOCS_DIR%\*" "%DOCS_DIR%\"

REM Backup original files
echo Backing up original files...
copy "%SRC_DIR%\knowledge.ts" "%SRC_DIR%\knowledge.ts.bak"
copy "%SRC_DIR%\index.ts" "%SRC_DIR%\index.ts.bak"

REM Create temporary files with the updates
echo Creating update files...

REM Create knowledge.ts update file
echo import fs from 'fs'; > "%TEMP%\knowledge_imports.txt"
echo import path from 'path'; >> "%TEMP%\knowledge_imports.txt"
echo. >> "%TEMP%\knowledge_imports.txt"
type "%SRC_DIR%\knowledge.ts" >> "%TEMP%\knowledge_imports.txt"

REM Add getAnchorDocumentation function
echo. >> "%TEMP%\knowledge_imports.txt"
echo /**>> "%TEMP%\knowledge_imports.txt"
echo  * Get Anchor documentation content>> "%TEMP%\knowledge_imports.txt"
echo  */>> "%TEMP%\knowledge_imports.txt"
echo export function getAnchorDocumentation(docPath: string): string {>> "%TEMP%\knowledge_imports.txt"
echo   // Base directory for Anchor documentation>> "%TEMP%\knowledge_imports.txt"
echo   const baseDir = path.resolve(__dirname, '../docs/anchor');>> "%TEMP%\knowledge_imports.txt"
echo.  >> "%TEMP%\knowledge_imports.txt"
echo   let filePath: string;>> "%TEMP%\knowledge_imports.txt"
echo.  >> "%TEMP%\knowledge_imports.txt"
echo   if (!docPath ^|^| docPath === '') {>> "%TEMP%\knowledge_imports.txt"
echo     // Main documentation page>> "%TEMP%\knowledge_imports.txt"
echo     filePath = path.join(baseDir, 'index.html');>> "%TEMP%\knowledge_imports.txt"
echo   } else if (docPath.includes('/')) {>> "%TEMP%\knowledge_imports.txt"
echo     // Section with page>> "%TEMP%\knowledge_imports.txt"
echo     filePath = path.join(baseDir, `${docPath.replace('/', '-')}.html`);>> "%TEMP%\knowledge_imports.txt"
echo   } else {>> "%TEMP%\knowledge_imports.txt"
echo     // Just a section>> "%TEMP%\knowledge_imports.txt"
echo     filePath = path.join(baseDir, `${docPath}.html`);>> "%TEMP%\knowledge_imports.txt"
echo   }>> "%TEMP%\knowledge_imports.txt"
echo.  >> "%TEMP%\knowledge_imports.txt"
echo   try {>> "%TEMP%\knowledge_imports.txt"
echo     if (fs.existsSync(filePath)) {>> "%TEMP%\knowledge_imports.txt"
echo       return fs.readFileSync(filePath, 'utf8');>> "%TEMP%\knowledge_imports.txt"
echo     } else {>> "%TEMP%\knowledge_imports.txt"
echo       return `^<html^>^<body^>^<h1^>Documentation Not Found^</h1^>^<p^>The requested Anchor documentation page "${docPath}" was not found.^</p^>^</body^>^</html^>`;>> "%TEMP%\knowledge_imports.txt"
echo     }>> "%TEMP%\knowledge_imports.txt"
echo   } catch (error) {>> "%TEMP%\knowledge_imports.txt"
echo     console.error(`Error reading Anchor documentation: ${error}`);>> "%TEMP%\knowledge_imports.txt"
echo     return `^<html^>^<body^>^<h1^>Error^</h1^>^<p^>An error occurred while reading the Anchor documentation: ${error}^</p^>^</body^>^</html^>`;>> "%TEMP%\knowledge_imports.txt"
echo   }>> "%TEMP%\knowledge_imports.txt"
echo }>> "%TEMP%\knowledge_imports.txt"

REM Copy the updated knowledge.ts file
copy "%TEMP%\knowledge_imports.txt" "%SRC_DIR%\knowledge.ts"

REM Update index.ts to include getAnchorDocumentation in the import
powershell -Command "(Get-Content '%SRC_DIR%\index.ts') -replace 'import \{ getRepositoryInfo, getComponentInfo, getRecommendations, searchKnowledgeBase \} from ''./knowledge.js'';', 'import { getRepositoryInfo, getComponentInfo, getRecommendations, searchKnowledgeBase, getAnchorDocumentation } from ''./knowledge.js'';' | Set-Content '%SRC_DIR%\index.ts'"

REM Create a file with the repository info to add
echo case 'anchor': > "%TEMP%\repo_info.txt"
echo   return { >> "%TEMP%\repo_info.txt"
echo     name: 'Anchor Framework', >> "%TEMP%\repo_info.txt"
echo     url: 'https://github.com/coral-xyz/anchor', >> "%TEMP%\repo_info.txt"
echo     description: 'Anchor is a framework for Solana''s Sealevel runtime providing several convenient developer tools for writing smart contracts.', >> "%TEMP%\repo_info.txt"
echo     key_components: [ >> "%TEMP%\repo_info.txt"
echo       { >> "%TEMP%\repo_info.txt"
echo         name: 'lang', >> "%TEMP%\repo_info.txt"
echo         description: 'Rust eDSL for writing Solana programs', >> "%TEMP%\repo_info.txt"
echo         potential_use: 'Simplified development of Solana programs with built-in security features' >> "%TEMP%\repo_info.txt"
echo       }, >> "%TEMP%\repo_info.txt"
echo       { >> "%TEMP%\repo_info.txt"
echo         name: 'client', >> "%TEMP%\repo_info.txt"
echo         description: 'TypeScript and Rust client libraries', >> "%TEMP%\repo_info.txt"
echo         potential_use: 'Strongly typed interfaces for interacting with Anchor programs' >> "%TEMP%\repo_info.txt"
echo       }, >> "%TEMP%\repo_info.txt"
echo       { >> "%TEMP%\repo_info.txt"
echo         name: 'spl', >> "%TEMP%\repo_info.txt"
echo         description: 'CPI clients for SPL programs', >> "%TEMP%\repo_info.txt"
echo         potential_use: 'Easy integration with SPL tokens and other SPL programs' >> "%TEMP%\repo_info.txt"
echo       }, >> "%TEMP%\repo_info.txt"
echo       { >> "%TEMP%\repo_info.txt"
echo         name: 'cli', >> "%TEMP%\repo_info.txt"
echo         description: 'Command line interface for managing Anchor projects', >> "%TEMP%\repo_info.txt"
echo         potential_use: 'Streamlined workflow for building, testing, and deploying Solana programs' >> "%TEMP%\repo_info.txt"
echo       } >> "%TEMP%\repo_info.txt"
echo     ], >> "%TEMP%\repo_info.txt"
echo     status: 'Active development' >> "%TEMP%\repo_info.txt"
echo   }; >> "%TEMP%\repo_info.txt"

REM Add the repository info to knowledge.ts
powershell -Command "(Get-Content '%SRC_DIR%\knowledge.ts') -replace 'switch \(repo\) \{', 'switch (repo) {`n    %REPO_INFO%' | Set-Content '%SRC_DIR%\knowledge.ts'"
powershell -Command "$content = Get-Content '%TEMP%\repo_info.txt' -Raw; $file = Get-Content '%SRC_DIR%\knowledge.ts' -Raw; $file = $file -replace '%REPO_INFO%', $content; Set-Content -Path '%SRC_DIR%\knowledge.ts' -Value $file"

REM Create a file with the search info to add
echo // Search Anchor documentation > "%TEMP%\search_info.txt"
echo if (query.toLowerCase().includes('anchor')) { >> "%TEMP%\search_info.txt"
echo   results.push({ >> "%TEMP%\search_info.txt"
echo     type: 'documentation', >> "%TEMP%\search_info.txt"
echo     title: 'Anchor Framework Documentation', >> "%TEMP%\search_info.txt"
echo     description: 'Complete documentation for the Anchor Framework', >> "%TEMP%\search_info.txt"
echo     url: 'solana-hft://docs/anchor', >> "%TEMP%\search_info.txt"
echo     relevance: 0.9 >> "%TEMP%\search_info.txt"
echo   }); >> "%TEMP%\search_info.txt"
echo } >> "%TEMP%\search_info.txt"
echo. >> "%TEMP%\search_info.txt"

REM Add the search info to knowledge.ts
powershell -Command "(Get-Content '%SRC_DIR%\knowledge.ts') -replace 'return results;', '%SEARCH_INFO%  return results;' | Set-Content '%SRC_DIR%\knowledge.ts'"
powershell -Command "$content = Get-Content '%TEMP%\search_info.txt' -Raw; $file = Get-Content '%SRC_DIR%\knowledge.ts' -Raw; $file = $file -replace '%SEARCH_INFO%', $content; Set-Content -Path '%SRC_DIR%\knowledge.ts' -Value $file"

REM Create a file with the resources to add
echo { > "%TEMP%\resources.txt"
echo   uri: `solana-hft://repositories/anchor`, >> "%TEMP%\resources.txt"
echo   name: `Anchor Framework`, >> "%TEMP%\resources.txt"
echo   mimeType: 'application/json', >> "%TEMP%\resources.txt"
echo   description: 'Information about the Anchor Framework for Solana development', >> "%TEMP%\resources.txt"
echo }, >> "%TEMP%\resources.txt"
echo { >> "%TEMP%\resources.txt"
echo   uri: `solana-hft://docs/anchor`, >> "%TEMP%\resources.txt"
echo   name: `Anchor Documentation`, >> "%TEMP%\resources.txt"
echo   mimeType: 'text/html', >> "%TEMP%\resources.txt"
echo   description: 'Complete documentation for the Anchor Framework', >> "%TEMP%\resources.txt"
echo }, >> "%TEMP%\resources.txt"

REM Add the resources to index.ts
powershell -Command "(Get-Content '%SRC_DIR%\index.ts') -replace 'resources: \[', 'resources: [`n        %RESOURCES%' | Set-Content '%SRC_DIR%\index.ts'"
powershell -Command "$content = Get-Content '%TEMP%\resources.txt' -Raw; $file = Get-Content '%SRC_DIR%\index.ts' -Raw; $file = $file -replace '%RESOURCES%', $content; Set-Content -Path '%SRC_DIR%\index.ts' -Value $file"

REM Create a file with the resource templates to add
echo { > "%TEMP%\resource_templates.txt"
echo   uriTemplate: 'solana-hft://docs/anchor/{section}', >> "%TEMP%\resource_templates.txt"
echo   name: 'Anchor Documentation Section', >> "%TEMP%\resource_templates.txt"
echo   mimeType: 'text/html', >> "%TEMP%\resource_templates.txt"
echo   description: 'Documentation section for the Anchor Framework', >> "%TEMP%\resource_templates.txt"
echo }, >> "%TEMP%\resource_templates.txt"
echo { >> "%TEMP%\resource_templates.txt"
echo   uriTemplate: 'solana-hft://docs/anchor/{section}/{page}', >> "%TEMP%\resource_templates.txt"
echo   name: 'Anchor Documentation Page', >> "%TEMP%\resource_templates.txt"
echo   mimeType: 'text/html', >> "%TEMP%\resource_templates.txt"
echo   description: 'Specific documentation page for the Anchor Framework', >> "%TEMP%\resource_templates.txt"
echo }, >> "%TEMP%\resource_templates.txt"

REM Add the resource templates to index.ts
powershell -Command "(Get-Content '%SRC_DIR%\index.ts') -replace 'resourceTemplates: \[', 'resourceTemplates: [`n          %RESOURCE_TEMPLATES%' | Set-Content '%SRC_DIR%\index.ts'"
powershell -Command "$content = Get-Content '%TEMP%\resource_templates.txt' -Raw; $file = Get-Content '%SRC_DIR%\index.ts' -Raw; $file = $file -replace '%RESOURCE_TEMPLATES%', $content; Set-Content -Path '%SRC_DIR%\index.ts' -Value $file"

REM Create a file with the handler code to add
echo // Handle Anchor documentation > "%TEMP%\handler.txt"
echo if (uri.startsWith('solana-hft://docs/anchor/')) { >> "%TEMP%\handler.txt"
echo   const path = uri.replace('solana-hft://docs/anchor/', ''); >> "%TEMP%\handler.txt"
echo   return { >> "%TEMP%\handler.txt"
echo     contents: [ >> "%TEMP%\handler.txt"
echo       { >> "%TEMP%\handler.txt"
echo         uri: request.params.uri, >> "%TEMP%\handler.txt"
echo         mimeType: 'text/html', >> "%TEMP%\handler.txt"
echo         text: getAnchorDocumentation(path), >> "%TEMP%\handler.txt"
echo       }, >> "%TEMP%\handler.txt"
echo     ], >> "%TEMP%\handler.txt"
echo   }; >> "%TEMP%\handler.txt"
echo } >> "%TEMP%\handler.txt"
echo. >> "%TEMP%\handler.txt"
echo // Handle Anchor main documentation page >> "%TEMP%\handler.txt"
echo if (uri === 'solana-hft://docs/anchor') { >> "%TEMP%\handler.txt"
echo   return { >> "%TEMP%\handler.txt"
echo     contents: [ >> "%TEMP%\handler.txt"
echo       { >> "%TEMP%\handler.txt"
echo         uri: request.params.uri, >> "%TEMP%\handler.txt"
echo         mimeType: 'text/html', >> "%TEMP%\handler.txt"
echo         text: getAnchorDocumentation(''), >> "%TEMP%\handler.txt"
echo       }, >> "%TEMP%\handler.txt"
echo     ], >> "%TEMP%\handler.txt"
echo   }; >> "%TEMP%\handler.txt"
echo } >> "%TEMP%\handler.txt"
echo. >> "%TEMP%\handler.txt"

REM Add the handler code to index.ts
powershell -Command "(Get-Content '%SRC_DIR%\index.ts') -replace 'throw new McpError', '%HANDLER%throw new McpError' | Set-Content '%SRC_DIR%\index.ts'"
powershell -Command "$content = Get-Content '%TEMP%\handler.txt' -Raw; $file = Get-Content '%SRC_DIR%\index.ts' -Raw; $file = $file -replace '%HANDLER%', $content; Set-Content -Path '%SRC_DIR%\index.ts' -Value $file"

REM Create a file with the tool to add
echo { > "%TEMP%\tool.txt"
echo   name: 'get_anchor_documentation', >> "%TEMP%\tool.txt"
echo   description: 'Get Anchor Framework documentation', >> "%TEMP%\tool.txt"
echo   inputSchema: { >> "%TEMP%\tool.txt"
echo     type: 'object', >> "%TEMP%\tool.txt"
echo     properties: { >> "%TEMP%\tool.txt"
echo       path: { >> "%TEMP%\tool.txt"
echo         type: 'string', >> "%TEMP%\tool.txt"
echo         description: 'Documentation path (e.g., "installation", "basics/pda")', >> "%TEMP%\tool.txt"
echo       }, >> "%TEMP%\tool.txt"
echo     }, >> "%TEMP%\tool.txt"
echo     required: [], >> "%TEMP%\tool.txt"
echo   }, >> "%TEMP%\tool.txt"
echo }, >> "%TEMP%\tool.txt"

REM Add the tool to index.ts
powershell -Command "(Get-Content '%SRC_DIR%\index.ts') -replace 'tools: \[', 'tools: [`n        %TOOL%' | Set-Content '%SRC_DIR%\index.ts'"
powershell -Command "$content = Get-Content '%TEMP%\tool.txt' -Raw; $file = Get-Content '%SRC_DIR%\index.ts' -Raw; $file = $file -replace '%TOOL%', $content; Set-Content -Path '%SRC_DIR%\index.ts' -Value $file"

REM Create a file with the case to add
echo case 'get_anchor_documentation': { > "%TEMP%\case.txt"
echo   const { path = '' } = request.params.arguments as { path?: string }; >> "%TEMP%\case.txt"
echo   return { >> "%TEMP%\case.txt"
echo     content: [ >> "%TEMP%\case.txt"
echo       { >> "%TEMP%\case.txt"
echo         type: 'text', >> "%TEMP%\case.txt"
echo         text: getAnchorDocumentation(path), >> "%TEMP%\case.txt"
echo       }, >> "%TEMP%\case.txt"
echo     ], >> "%TEMP%\case.txt"
echo   }; >> "%TEMP%\case.txt"
echo } >> "%TEMP%\case.txt"
echo. >> "%TEMP%\case.txt"

REM Add the case to index.ts
powershell -Command "(Get-Content '%SRC_DIR%\index.ts') -replace 'default:', '%CASE%default:' | Set-Content '%SRC_DIR%\index.ts'"
powershell -Command "$content = Get-Content '%TEMP%\case.txt' -Raw; $file = Get-Content '%SRC_DIR%\index.ts' -Raw; $file = $file -replace '%CASE%', $content; Set-Content -Path '%SRC_DIR%\index.ts' -Value $file"

REM Rebuild the server
echo Rebuilding the server...
cd "%MCP_SERVER_DIR%"
call npm run build

echo Installation complete!
echo You can now access the Anchor documentation through the MCP server.
echo - Repository information: solana-hft://repositories/anchor
echo - Main documentation page: solana-hft://docs/anchor
echo - Specific documentation sections: solana-hft://docs/anchor/installation
echo - Specific documentation pages: solana-hft://docs/anchor/basics/pda