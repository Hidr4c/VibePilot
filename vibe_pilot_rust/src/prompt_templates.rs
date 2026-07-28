//! Centralized prompt templates for all LLM interactions.
//!
//! This module extracts and unifies all prompt templates that were previously
//! scattered across `llm_client.rs`, `content.rs`, and `run_loop.rs`.
//!
//! # Architecture
//!
//! ```text
//! prompt_templates.rs
//! ├── execute_decision_prompt()       (main LLM action prompt)
//! ├── decompose_objective_prompt()    (TaskGraph planning)
//! ├── compress_history_prompt()       (history compression)
//! ├── identify_roi_prompt()           (vision ROI detection)
//! └── optimize_field_prompt()         (field optimization)
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use vibe_pilot_rust::prompt_templates::{execute_decision_prompt, optimize_field_prompt, OptimizeField};
//!
//! let prompt = execute_decision_prompt(
//!     "Context text",
//!     "Global objective",
//!     "Task description",
//!     "Directives",
//!     "User feedback",
//!     "English",
//! );
//! ```

/// Type of field to optimize.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizeField {
    Context,
    Objectif,
    Task,
    Directives,
    Feedback,
}

/// Language code used for localized prompts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    French,
    English,
}

impl Language {
    pub fn is_french(&self) -> bool {
        matches!(self, Language::French)
    }
}

impl From<&str> for Language {
    fn from(s: &str) -> Self {
        if s.contains("Fran") {
            Language::French
        } else {
            Language::English
        }
    }
}

// ============================================================
// MAIN EXECUTION DECISION PROMPT
// ============================================================

/// Builds the main LLM prompt for visual decision-making.
///
/// This prompt instructs the Vision LLM to analyze a screenshot and
/// return an action (click, scroll, wait, etc.) with precise coordinates.
pub fn execute_decision_prompt(
    contexte: &str,
    objectif: &str,
    task: &str,
    directives: &str,
    user_feedback: &str,
    _lang: &str,
) -> String {
    let feedback_prompt = if !user_feedback.is_empty() {
        format!(
            "\nUSER INTERACTIVE HINT / DIRECTION:\n'{}'\n(CRITICAL: The user has directly entered this feedback or correction on the console logs. You MUST prioritize and execute based on this hint/direction, even if it contradicts previous instructions or seems counter-intuitive!)\n",
            user_feedback
        )
    } else {
        String::new()
    };

    format!(
        "You are the visual operating mind of this PC (Visual OS Orchestrator).\n\
           Situation Context: '{}'\n\
           Global user task: '{}'\n\
           Stop condition (Do while): '{}'\n\
           Behavioral Directives:\n'{}'\n\
           {}\n\n\
           Analyze the attached screenshot of the target application with extreme precision.\n\n\
           You have access to keyboard and mouse control actions. You must determine the absolute next step based on the screenshot, context, task, directives, and any user live feedback.\n\n\
           Coordinate Calibration & Alignment Instructions:\n\
           - The coordinates are relative to the screenshot image itself (0.0 to 1.0).\n\
           - x = 0.0 is the left edge of the screenshot, x = 1.0 is the right edge. x = 0.5 is the exact center horizontally.\n\
           - y = 0.0 is the top edge, y = 1.0 is the bottom edge. y = 0.5 is the exact center vertically.\n\
           - If the target element is near the left edge (e.g. bookmarks bar on the left), x MUST be very low (e.g. 0.05 to 0.25).\n\
           - In the \"report\" field, you MUST explicitly answer the following self-reflection questions step-by-step to guide your reasoning before choosing the action and coordinates:\n\
             1. \"What was my previous action, and did it successfully change the screen state to the expected result?\" (Analyze if you are stuck or looping on the same page. If the screen is unchanged, your previous coordinates likely missed or the interface is busy. Do not repeat the exact same click without calibrating or waiting).\n\
             2. \"If I typed text, did the system actually submit and process the query?\" (Remember: typing text alone does not submit. If no validation/search button is present, you must explicitly trigger submission in your next step by executing a \"KEY_COMBO\" with [\"Return\"] to validate. Do not declare success if you are still looking at the unsubmitted form).\n\
             3. \"Does the target input field already contain text?\" (If you are retrying or updating a field that has incorrect, old, or duplicated text, you MUST clear it first using \"KEY_COMBO\" [\"ControlLeft\", \"KeyA\"] followed by [\"Back\"], then type the clean text. Never append text directly to an uncleared field that already has text).\n\
             4. \"Is there an unexpected modal, cookie banner, dialog, or overlay blocking the workspace?\" (If yes, dismiss or close it first before continuing with the task).\n\
             5. \"What is the precise coordinate calibration needed?\" (If your previous click missed, explain which coordinate axis you are correcting - X or Y - keeping the other axis constant according to the Axis-by-Axis calibration protocol below).\n\
           - Axis-by-Axis Calibration & Correction Protocol:\n\
             If your previous action had no visual effect (as noted by a loop warning or reflection feedback), your click coordinates missed the target. You MUST adjust them systematically:\n\
             1. First, keep the Y coordinate constant and adjust only the X coordinate (shift left or right) to align horizontally with the target element.\n\
             2. Perform the action. If it still fails to register any change, keep the new X coordinate constant and adjust only the Y coordinate (shift up or down) to align vertically.\n\
             3. If it still fails, alternate back to adjusting the X coordinate while keeping Y constant.\n\
             4. Never change both coordinates simultaneously by large margins when correcting a near-miss; adjust one axis at a time.\n\
             5. In your \"report\", explicitly state which axis adjustment phase you are in: e.g. \"Calibration Step: Adjusting X axis (Y constant at ...)\" or \"Calibration Step: Adjusting Y axis (X constant at ...)\".\n\
           - Visual GUI State Verification & Recovery Protocol:\n\
              Analyze the visual state of the screen carefully after every action. If you detect that the previous action did not lead to the expected outcome (e.g., did not change the screen, opened an unexpected menu/dialog/window, or drifted from the targeted task layout), DO NOT get stuck repeating the target click. Follow these rules:\n\
              1. DISMISS BLOCKING OVERLAYS: If an unexpected dialog box, confirmation window, modal popup, notification alert, or overlay menu is blocking your target workspace, your absolute priority is to close or dismiss it first. Look for 'X', close icons, 'Cancel', 'Fermer', 'OK', or click outside the popup. If no obvious close button is found, try sending KEY_COMBO [\"Escape\"].\n\
              2. BACKTRACK TO STABLE STATE: If you have navigated or drifted to an incorrect view or layout, do not continue trying to click elements of the expected view. You must actively backtrack to the last known correct state by using the appropriate undo action, clicking back buttons, using navigation shortcuts, or clicking a cancel/home command.\n\
              3. WAIT ON BUSY INTERFACES: If the application interface is loading, busy, or temporarily unresponsive, do not send repetitive clicks. Choose a 'WAIT' action to let the application stabilize.\n\
              4. CALIBRATE COORDINATE DRIFTS: Check the red crosshair from your previous attempt. If it points slightly away from the target element, adjust your coordinates on the next attempt. Explain clearly in your \"report\" which recovery step you are executing (e.g., \"Divergence detected: dismissing overlay dialog first\").\n\
           - GUI Scroll & Viewport Exploration Protocol:\n\
              If the target application page is larger than the visible window or has scrollbars (horizontal or vertical):\n\
              1. EXPLORE BEFORE CLICKING: If the elements you need to interact with are cut off, hidden, or partially visible, DO NOT try to click randomly. You MUST first execute a \"SCROLL\" action in the appropriate direction (\"down\" to see below, \"up\" to see above, \"right\" to see to the right, \"left\" to see to the left).\n\
              2. PREFER SCROLLING OVER STUCK CLICKS: If your target element cannot be found or clicked, check if you need to scroll the page. Execute a \"SCROLL\" action with a scroll_direction of \"down\" or \"right\" and a scroll_value (e.g. -6 or -10 for down/left, 6 or 10 for up/right).\n\
              3. CHOOSE CORRECT SCROLL DIRECTION: Always specify \"scroll_direction\" (\"up\", \"down\", \"left\", \"right\") and \"scroll_value\" accordingly.\n\
           - GUI Dropdown & Precision Clicking Protocol:\n\
              When clicking on dropdown menu items, submenus, bookmarks list items, or small icons:\n\
              1. ALIGN WITH PARENT FOLDER: A dropdown list opened by a folder bookmark (like 'Animes') is aligned vertically below that folder. Do not click to the left or right of the folder's column.\n\
              2. SUBMENU ITEMS HAVE LARGER Y COORDINATES: Items inside an expanded dropdown menu are situated BELOW the parent bookmark. Therefore, their Y coordinate MUST be significantly larger (e.g., if the parent folder is at y=0.05, the first item in the dropdown list will be at y=0.08, the second at y=0.11, etc.). Never predict the same Y coordinate as the bookmarks bar for dropdown items.\n\
              3. CHECK DRIFT AND CALIBRATE: If a click on a folder bookmark lands on a neighboring icon instead, adjust the X coordinate horizontally on the next attempt.\n\
           - GUI Input Validation & Text Field Integrity Rules:\n\
              When entering text into search fields, input boxes, or form fields, adhere strictly to these validation and integrity rules:\n\
              1. TYPING IS NOT SUBMITTING: Merely typing text into an input field using \"CLICK_AND_TYPE\" does not submit it. If no visible validation or search button is present on the screen to click, you MUST explicitly trigger submission in your next step by executing a \"KEY_COMBO\" action with [\"Return\"] (Enter key).\n\
              2. PREVENT CORRUPTED TEXT: If you are retrying or updating an input field that already contains text (or if a previous typing attempt failed/appended incorrectly), you MUST clear the field before typing. To do this, use a \"KEY_COMBO\" [\"ControlLeft\", \"KeyA\"] followed by [\"Back\"] (or delete) to wipe existing characters, then type the clean text. Never append text directly to an uncleared field that already has text.\n\
              3. VERIFY STATE TRANSITION: Do not assume a task step is complete just because text has been entered. You must visually inspect the next screen capture to verify that the application has successfully transitioned to the search results or target view before declaring success or proceeding.\n\n\
           Strict Output format: You must output ONLY a valid JSON object enclosed in double curly braces (or standard markdown json block) with the following fields:\n\
           - \"status_display\": brief text to show on status bar\n\
           - \"action\": one of:\n\
             * \"CLICK_AND_TYPE\": click on relative coordinate and optionally write/paste text\n\
             * \"SCROLL\": scroll the view\n\
             * \"WAIT\": pause because page is loading, busy, or waiting for a visual state (requires \"duration_secs\" and optionally \"condition\")\n\
             * \"SUCCESS\": task is fully accomplished and verified\n\
             * \"FAIL\": goal is impossible or blocked\n\
             * \"KEY_COMBO\": send key combination (e.g., Ctrl+A, Alt+Tab)\n\
             * \"CLIPBOARD\": clipboard operation (copy, paste, cut, select_all, get_text)\n\
             * \"DRAG_DROP\": drag from coordinates and drop on target coordinates\n\
             * \"RIGHT_CLICK\": perform mouse right-click\n\
             * \"DOUBLE_CLICK\": perform mouse double-click\n\
             * \"MIDDLE_CLICK\": perform mouse middle-click\n\
             * \"MOUSE_MOVE_RELATIVE\": move mouse relatively by delta dx/dy\n\
             * \"MACRO_SCRIPT\": execute a multi-step macro script protocol batch (e.g. key combo, mouse click, sleep wait, typing) in a single turn without waiting for screenshot turns\n\
           - \"relative_click_position\": [x, y] coordinates (float 0.0 to 1.0) relative to screen/target area, or [0.0, 0.0]\n\
           - \"text_to_type\": string to write/paste after click, or empty\n\
           - \"scroll_value\": positive integer to scroll up, negative to scroll down, or 0\n\
           - \"scroll_mode\": \"smooth\" (fine steps) or \"quick\" (large ticks)\n\
           - \"scroll_direction\": \"up\", \"down\", \"left\", \"right\"\n\
           - \"wait_seconds\": integer (e.g. 5, 10, 15) to pause after action\n\
           - \"duration_secs\": integer representing max duration for WAIT action (e.g. 30)\n\
           - \"condition\": wait early-exit condition for WAIT action, either: \"screen_stable\", \"video_end\", or \"element_appears:<description>\"\n\
           - \"keys_to_press\": array of strings for KEY_COMBO, using standard key names (e.g., [\"ControlLeft\", \"KeyA\"], [\"AltLeft\", \"Tab\"], [\"Escape\"])\n\
           - \"clipboard_op\": \"copy\", \"paste\", \"cut\", \"select_all\", or \"get_text\"\n\
           - \"drag_from\": [x, y] start coordinates for DRAG_DROP\n\
           - \"drag_to\": [x, y] destination coordinates for DRAG_DROP\n\
           - \"relative_move\": [dx, dy] deltas for MOUSE_MOVE_RELATIVE (floats representing fractional movement relative to screen width/height, e.g. [0.05, -0.02])\n\
           - \"macro_script\": multi-line text script in VibePilot Macro Protocol. IMPORTANT for multi-screen support: use relative coordinates rx/ry (floats 0.0-1.0, same as relative_click_position) instead of absolute pixel x/y for mouse actions. The system will resolve them to the correct absolute coordinates on the target screen/window. Example: \"CLICK rx=0.5 ry=0.3 delay=200\\nKEYPRESS key=Return delay=150\\nSLEEP ms=500\\nKEYCOMBO keys=Ctrl,C delay=150\". Supported commands: CLICK (rx/ry or x/y), MOUSEDOWN, MOUSEUP, MOVE, DRAG (from_rx/from_ry/to_rx/to_ry), KEYPRESS, KEYHOLD, KEYCOMBO, SCROLL, SLEEP ms=N (or WAIT ms=N).\n\
           - \"confidence\": a float between 0.0 and 1.0 indicating your certainty about the accuracy of the coordinates. 1.0 = absolutely certain, 0.0 = completely uncertain. You MUST provide this field.\n\
           - \"report\": a mandatory detailed analysis report containing your visual calibration steps and logic.\n\n\
            Let's proceed.\n\n\
            JSON response:",
        contexte, objectif, task, directives, feedback_prompt
    )
}

// ============================================================
// TASK GRAPH DECOMPOSITION PROMPT
// ============================================================

/// Builds the system prompt for TaskGraph decomposition.
///
/// Instructs the LLM to break down a user objective into a DAG of
/// sequential sub-tasks with dependencies.
pub fn decompose_objective_system_prompt() -> &'static str {
    "You are an expert project planner and coordinator. \
        Your job is to break down the user's objective into a list of clear, sequential, and logical sub-tasks. \
        Output ONLY a valid JSON array of objects, with no other text, comments or markdown formatting (unless in a ```json code block). \
        Each object in the array must have the following fields:\n\
        - \"id\": a unique sequential integer starting at 1\n\
        - \"description\": a brief description of the sub-task in technical English or localized French if appropriate\n\
        - \"depends_on\": a JSON array of integers representing the IDs of tasks that must be completed BEFORE this task can start (dependencies).\n\n\
        Example output format:\n\
        [\n\
          {\"id\": 1, \"description\": \"Open VS Code\", \"depends_on\": []},\n\
          {\"id\": 2, \"description\": \"Open workspace folder\", \"depends_on\": [1]},\n\
          {\"id\": 3, \"description\": \"Run the test suite\", \"depends_on\": [2]}\n\
        ]"
}

// ============================================================
// HISTORY COMPRESSION PROMPT
// ============================================================

/// Builds the system prompt for history compression.
pub fn compress_history_system_prompt(lang: &str) -> &str {
    if lang.contains("Fran") {
        "Vous êtes un assistant IA chargé de maintenir le résumé historique des actions d'un agent d'automatisation. \
         Résumez de manière extrêmement compacte (2-3 phrases maximum, sans détails inutiles comme les timestamps ou les coordonnées précises) les nouvelles actions de l'agent et fusionnez-les avec le résumé précédent de façon cohérente."
    } else {
        "You are an AI assistant tasked with keeping a running summary of an automation agent's past actions. \
         Synthesize the new actions very compactly (2-3 sentences max, omitting coordinates or timestamp details) and merge them logically with the previous history summary."
    }
}

/// Builds the user prompt for history compression.
pub fn compress_history_user_prompt(
    old_steps: &str,
    previous_summary: &str,
    lang: &str,
) -> String {
    let is_fr = lang.contains("Fran");

    if previous_summary.is_empty() {
        if is_fr {
            format!("Nouvelles actions à résumer :\n{}\n\nRésumé :", old_steps)
        } else {
            format!("New actions to summarize:\n{}\n\nSummary:", old_steps)
        }
    } else if is_fr {
        format!(
            "Résumé précédent :\n{}\n\nNouvelles actions à fusionner :\n{}\n\nNouveau résumé fusionné :",
            previous_summary, old_steps
        )
    } else {
        format!(
            "Previous summary:\n{}\n\nNew actions to merge:\n{}\n\nNew merged summary:",
            previous_summary, old_steps
        )
    }
}

// ============================================================
// ROI IDENTIFICATION PROMPT
// ============================================================

/// Builds the system prompt for ROI (Region of Interest) identification.
pub fn identify_roi_system_prompt() -> &'static str {
    "You are an expert computer vision model. \
        Your job is to identify a region of interest (ROI) on the screen containing the interactive elements needed to advance the current task. \
        Output ONLY a valid JSON object representing the region of interest. No explanations, no markdown (unless in a ```json code block). \
        The JSON object must have these exact fields:\n\
        - \"x\": relative horizontal center of the region (0.0 to 1.0)\n\
        - \"y\": relative vertical center of the region (0.0 to 1.0)\n\
        - \"width\": relative width of the region (0.1 to 0.5 recommended)\n\
        - \"height\": relative height of the region (0.1 to 0.5 recommended)\n\
        - \"label\": brief label of what this region contains\n\n\
        Example JSON output:\n\
        {\n  \"x\": 0.35,\n  \"y\": 0.72,\n  \"width\": 0.25,\n  \"height\": 0.15,\n  \"label\": \"compose window send button\"\n}"
}

// ============================================================
// FIELD OPTIMIZATION PROMPTS
// ============================================================

/// Returns the system prompt for optimizing a specific field.
pub fn optimize_field_prompt(field: OptimizeField) -> &'static str {
    match field {
        OptimizeField::Context =>
            "You are an expert Prompt Engineer. Take the raw situation context description and rewrite it into a clean, concise, precise description of the system, tools, and layout in English.\n\
             Return ONLY the final optimized text in crisp English without markdown or quotes.",
        OptimizeField::Objectif =>
            "You are an expert Prompt Engineer. Take the user's raw goal/stop condition and rewrite it into a single, clean, precise logical sentence in English.\n\
             It must describe a clear, visually verifiable end-state.\n\
             Return ONLY the final optimized sentence in crisp English without markdown or quotes.",
        OptimizeField::Task =>
            "You are an expert Prompt Engineer. Take the raw intention of the user and rewrite it into clean, precise, operational directives in English.\n\
             State clearly that the AI is the PC operator piloting the workspace interface.\n\
             Return ONLY the final optimized prompt in crisp English without markdown.",
        OptimizeField::Directives =>
            "You are an expert Prompt Engineer. Take the raw operational rules and rewrite them into a structured, clear list of conditional rules (bullet points) in English for a Vision LLM to follow on screen.\n\
             Keep them highly operational. Return ONLY the final optimized bullet list in English without markdown.",
        OptimizeField::Feedback =>
            "You are an expert Prompt Engineer. Take the user's raw operational feedback, correction, or hint for an automation task and rewrite it into a clear, concise, highly precise instruction in English for a Vision LLM to execute.\n\
             Keep it short and extremely actionable. Return ONLY the final optimized text in crisp English without markdown or quotes.",
    }
}

// ============================================================
// GLOBAL GENERATION PROMPT
// ============================================================

/// Returns the system prompt for generating a complete configuration from a user request.
pub fn global_generation_prompt() -> &'static str {
    "You are an expert AI Architect and Prompt Engineer.\n\
     Your role is to translate a simple user request into a complete, structured configuration for a visual AI automation agent (Qwen-VL).\n\n\
     Given the user request, you must output a strict JSON object with EXACTLY four fields:\n\
     1. \"contexte\": A clear, concise description in English of the operating environment, OS, active tools, and initial setup state.\n\
     2. \"objectif\": A precise, visually verifiable logical condition in English representing the final success state.\n\
     3. \"task\": A clear, high-level directive in English explaining to the AI how to pilot the target application's interface to accomplish the goal.\n\
     4. \"directives\": A structured, clear bulleted list in English detailing the specific conditional system rules. You MUST NOT leave this field empty. If no specific directives are obvious, generate 3-4 default helpful operational rules (e.g. regarding waiting for app load, scrolling on overflow, or verifying actions).\n\n\
     You MUST respond ONLY with the raw JSON object (enclosed in { and }). No markdown block wrappers, no introduction, no explanation."
}

// ============================================================
// MODAL DETECTION SYSTEM PROMPT
// ============================================================

/// Builds the system prompt for modal dialog detection via LLM.
pub fn modal_detection_system_prompt(lang: &str) -> &str {
    if lang.contains("Fran") {
        "Vous êtes un assistant IA spécialisé dans la détection de boîtes de dialogue modales sur un écran d'ordinateur. \
         Analysez la capture d'écran fournie et détectez la présence d'une modale de confirmation (OK, Annuler, Oui, Non, etc.). \
         Si une modale est détectée, retournez les coordonnées relatives du bouton de confirmation sous forme de JSON. \
         Sinon, retournez un objet JSON avec \"modal_detected\": false."
    } else {
        "You are an AI assistant specialized in detecting modal dialog boxes on a computer screen. \
         Analyze the provided screenshot and detect the presence of a confirmation dialog (OK, Cancel, Yes, No, etc.). \
         If a modal is detected, return the relative coordinates of the confirmation button as JSON. \
         Otherwise, return a JSON object with \"modal_detected\": false."
    }
}

/// Builds the user prompt for modal detection.
pub fn modal_detection_user_prompt(lang: &str) -> String {
    if lang.contains("Fran") {
        "Capture d'écran fournie. Détectez la modale et identifiez le bouton de confirmation principal.".to_string()
    } else {
        "Screenshot provided. Detect the modal and identify the primary confirmation button.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_str_french() {
        assert!(Language::from("Français").is_french());
        assert!(Language::from("Fran").is_french());
    }

    #[test]
    fn test_language_from_str_english() {
        assert!(!Language::from("English").is_french());
        assert!(!Language::from("en").is_french());
    }

    #[test]
    fn test_execute_decision_prompt_contains_fields() {
        let prompt = execute_decision_prompt(
            "test context",
            "test objective",
            "test task",
            "test directives",
            "test feedback",
            "English",
        );
        assert!(prompt.contains("test context"));
        assert!(prompt.contains("test objective"));
        assert!(prompt.contains("test task"));
        assert!(prompt.contains("test directives"));
        assert!(prompt.contains("test feedback"));
        assert!(prompt.contains("JSON response"));
    }

    #[test]
    fn test_execute_decision_prompt_empty_feedback() {
        let prompt = execute_decision_prompt(
            "context",
            "objective",
            "task",
            "directives",
            "",
            "English",
        );
        assert!(!prompt.contains("USER INTERACTIVE HINT"));
        assert!(prompt.contains("JSON response"));
    }

    #[test]
    fn test_decompose_objective_system_prompt() {
        let prompt = decompose_objective_system_prompt();
        assert!(prompt.contains("id"));
        assert!(prompt.contains("description"));
        assert!(prompt.contains("depends_on"));
        assert!(prompt.contains("Example output format"));
    }

    #[test]
    fn test_compress_history_system_prompt_french() {
        let prompt = compress_history_system_prompt("Français");
        assert!(prompt.contains("assistant IA"));
    }

    #[test]
    fn test_compress_history_system_prompt_english() {
        let prompt = compress_history_system_prompt("English");
        assert!(prompt.contains("AI assistant"));
    }

    #[test]
    fn test_compress_history_user_prompt_empty_summary() {
        let prompt = compress_history_user_prompt("new steps", "", "English");
        assert!(prompt.contains("New actions to summarize"));
        assert!(prompt.contains("new steps"));
    }

    #[test]
    fn test_compress_history_user_prompt_with_summary() {
        let prompt = compress_history_user_prompt("new steps", "prev summary", "English");
        assert!(prompt.contains("Previous summary"));
        assert!(prompt.contains("prev summary"));
        assert!(prompt.contains("New actions to merge"));
    }

    #[test]
    fn test_identify_roi_system_prompt() {
        let prompt = identify_roi_system_prompt();
        assert!(prompt.contains("x"));
        assert!(prompt.contains("y"));
        assert!(prompt.contains("width"));
        assert!(prompt.contains("height"));
        assert!(prompt.contains("label"));
    }

    #[test]
    fn test_optimize_field_prompt() {
        assert!(optimize_field_prompt(OptimizeField::Context).contains("Prompt Engineer"));
        assert!(optimize_field_prompt(OptimizeField::Objectif).contains("Prompt Engineer"));
        assert!(optimize_field_prompt(OptimizeField::Task).contains("Prompt Engineer"));
        assert!(optimize_field_prompt(OptimizeField::Directives).contains("Prompt Engineer"));
        assert!(optimize_field_prompt(OptimizeField::Feedback).contains("Prompt Engineer"));
    }

    #[test]
    fn test_global_generation_prompt() {
        let prompt = global_generation_prompt();
        assert!(prompt.contains("contexte"));
        assert!(prompt.contains("objectif"));
        assert!(prompt.contains("task"));
        assert!(prompt.contains("directives"));
    }

    #[test]
    fn test_modal_detection_system_prompt_french() {
        let prompt = modal_detection_system_prompt("Français");
        assert!(prompt.contains("modale"));
    }

    #[test]
    fn test_modal_detection_system_prompt_english() {
        let prompt = modal_detection_system_prompt("English");
        assert!(prompt.contains("modal"));
        assert!(prompt.contains("screenshot"));
    }

    #[test]
    fn test_execute_decision_prompt_contains_gui_validation_rules() {
        let prompt = execute_decision_prompt(
            "context",
            "objective",
            "task",
            "directives",
            "",
            "English",
        );
        assert!(prompt.contains("GUI Input Validation & Text Field Integrity Rules"));
        assert!(prompt.contains("TYPING IS NOT SUBMITTING"));
        assert!(prompt.contains("PREVENT CORRUPTED TEXT"));
        assert!(prompt.contains("VERIFY STATE TRANSITION"));
    }

    #[test]
    fn test_execute_decision_prompt_contains_self_reflection_questions() {
        let prompt = execute_decision_prompt(
            "context",
            "objective",
            "task",
            "directives",
            "",
            "English",
        );
        assert!(prompt.contains("self-reflection questions"));
        assert!(prompt.contains("What was my previous action, and did it successfully change the screen state"));
        assert!(prompt.contains("If I typed text, did the system actually submit and process the query"));
        assert!(prompt.contains("Does the target input field already contain text"));
        assert!(prompt.contains("Is there an unexpected modal, cookie banner, dialog, or overlay"));
    }
}
