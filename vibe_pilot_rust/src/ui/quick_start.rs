use crate::app::VibePilotApp;
use eframe::egui;

// ============================================================
// TAB 3: Help / Quick Start (demande générique)
// ============================================================
pub fn render_help_tab(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    let width = ui.available_width();

    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("frame_generateur")).on_hover_text(app.t("tip_generate_all"));
        });
        ui.separator();

        let hint_gen_desc = if app.current_config.langue == "Français" {
            "Saisissez une simple demande et l'IA va générer la configuration complète :"
        } else {
            "Enter a simple request and AI will generate the full configuration:"
        };
        ui.label(hint_gen_desc);
        ui.add_space(6.0);
        let hint_gen_ex = if app.current_config.langue == "Français" {
            "Exemple: 'Automatiser les tests d'extension VS Code pour atteindre 80% de couverture'"
        } else {
            "Example: 'Automate testing VS Code extension to reach 80% coverage'"
        };
        ui.add(egui::TextEdit::multiline(&mut app.current_config.demande_generique)
            .hint_text(hint_gen_ex)
            .desired_width(width)
            .desired_rows(4));

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let label_profile = if app.current_config.langue == "Français" { "Nom du profil à créer :" } else { "Profile name to create:" };
            let hint_profile = if app.current_config.langue == "Français" { "Nom du nouveau profil..." } else { "New profile name..." };
            ui.label(label_profile);
            ui.add(egui::TextEdit::singleline(&mut app.quick_start_profile_name).hint_text(hint_profile));
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(egui::RichText::new(app.t("btn_generer_tout")).color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(106, 27, 154))).on_hover_text(app.t("tip_generate_all")).clicked() {
                app.generate_prompts_from_request();
            }
        });
    });

    ui.add_space(12.0);

    // Help Text / Instructions
    ui.group(|ui| {
        let (help_title, help_step1, help_step2, help_step3, help_step4) = if app.current_config.langue == "Français" {
            (
                "Aide - Démarrage Rapide",
                "1. Sélectionnez un moteur d'IA et configurez l'URL de l'API et le Modèle.",
                "2. Sélectionnez les applications cibles à surveiller.",
                "3. Décrivez la tâche globale, les conditions d'arrêt et les règles du système dans l'éditeur de prompts.",
                "4. Utilisez les contrôles en bas pour démarrer ou suspendre la boucle d'automatisation."
            )
        } else {
            (
                "Quick Start Help",
                "1. Select an AI engine and configure the API URL and Model.",
                "2. Select the target applications you want to monitor.",
                "3. Describe the global task, stop conditions, and specific system rules in the Prompt Editor.",
                "4. Use the controls at the bottom to Start/Resume the automation loop."
            )
        };
        ui.label(help_title);
        ui.separator();
        ui.label(help_step1);
        ui.label(help_step2);
        ui.label(help_step3);
        ui.label(help_step4);
    });
}
