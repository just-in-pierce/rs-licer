use iced::widget::{button, checkbox, column, container, progress_bar, row, text, text_input};
use iced::{Alignment, Element, Length, Subscription, Task, Theme};
use rs_licer::{slice_with_progress, SlicerConfig};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

pub fn run_gui() -> iced::Result {
    iced::application("rs-licer", SlicerApp::update, SlicerApp::view)
        .theme(SlicerApp::theme)
        .subscription(SlicerApp::subscription)
        .run()
}

#[derive(Debug, Clone)]
pub enum Message {
    InputPathChanged(String),
    OutputDirChanged(String),
    PixelSizeChanged(String),
    LayerHeightChanged(String),
    ZeroSliceToggled(bool),
    DeleteBelowZeroToggled(bool),
    DeleteOutputDirToggled(bool),
    OpenOutputDirToggled(bool),
    BrowseFile,
    BrowseOutputDir,
    Slice,
    Tick,
}

pub struct SlicerApp {
    input_path: String,
    output_dir: String,
    pixel_size: String,
    layer_height: String,
    zero_slice_position: bool,
    delete_below_zero: bool,
    delete_output_dir: bool,
    open_output_dir: bool,
    is_processing: bool,
    progress: f32,
    status_message: String,
    progress_rx: Option<Receiver<(f32, String)>>,
    start_time: Option<Instant>,
    estimated_time: Option<String>,
}

impl Default for SlicerApp {
    fn default() -> Self {
        Self {
            input_path: String::new(),
            output_dir: "slices".to_string(),
            pixel_size: "33.3333".to_string(),
            layer_height: "20.0".to_string(),
            zero_slice_position: false,
            delete_below_zero: false,
            delete_output_dir: true,
            open_output_dir: true,
            is_processing: false,
            progress: 0.0,
            status_message: "Ready to slice".to_string(),
            progress_rx: None,
            start_time: None,
            estimated_time: None,
        }
    }
}

impl SlicerApp {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputPathChanged(value) => {
                self.input_path = value;
                Task::none()
            }
            Message::OutputDirChanged(value) => {
                self.output_dir = value;
                Task::none()
            }
            Message::PixelSizeChanged(value) => {
                self.pixel_size = value;
                Task::none()
            }
            Message::LayerHeightChanged(value) => {
                self.layer_height = value;
                Task::none()
            }
            Message::ZeroSliceToggled(value) => {
                self.zero_slice_position = value;
                Task::none()
            }
            Message::DeleteBelowZeroToggled(value) => {
                self.delete_below_zero = value;
                Task::none()
            }
            Message::DeleteOutputDirToggled(value) => {
                self.delete_output_dir = value;
                Task::none()
            }
            Message::OpenOutputDirToggled(value) => {
                self.open_output_dir = value;
                Task::none()
            }
            Message::BrowseFile => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("STL Files", &["stl"])
                    .pick_file()
                {
                    self.input_path = path.display().to_string();
                }
                Task::none()
            }
            Message::BrowseOutputDir => {
                if let Some(path) = rfd::FileDialog::new()
                    .pick_folder()
                {
                    self.output_dir = path.display().to_string();
                }
                Task::none()
            }
            Message::Slice => {
                if self.input_path.is_empty() {
                    self.status_message = "Please select an input file".to_string();
                    return Task::none();
                }

                let pixel_size = self.pixel_size.parse::<f32>().unwrap_or(33.3333);
                let layer_height = self.layer_height.parse::<f32>().unwrap_or(20.0);

                let config = SlicerConfig {
                    input_path: self.input_path.clone(),
                    output_dir: self.output_dir.clone(),
                    pixel_size_um: pixel_size,
                    layer_height_um: layer_height,
                    zero_slice_position: self.zero_slice_position,
                    delete_below_zero: self.delete_below_zero,
                    delete_output_dir: self.delete_output_dir,
                    open_output_dir: self.open_output_dir,
                };

                self.is_processing = true;
                self.progress = 0.0;
                self.status_message = "Starting...".to_string();
                self.start_time = Some(Instant::now());

                let (tx, rx) = channel();
                self.progress_rx = Some(rx);

                std::thread::spawn(move || {
                    slice_with_progress(config, Some(tx));
                });

                Task::none()
            }
            Message::Tick => {
                let mut should_finish = false;
                
                if let Some(ref rx) = self.progress_rx {
                    while let Ok((progress, message)) = rx.try_recv() {
                        self.progress = progress;
                        self.status_message = message;

                        if let Some(start) = self.start_time {
                            if progress > 0.0 && progress < 1.0 {
                                let elapsed = start.elapsed().as_secs_f32();
                                let total_estimated = elapsed / progress;
                                let remaining = total_estimated - elapsed;

                                let mins = (remaining / 60.0) as u32;
                                let secs = (remaining % 60.0) as u32;
                                self.estimated_time = Some(format!("{}m {}s", mins, secs));
                            } else if progress >= 1.0 {
                                should_finish = true;
                            }
                        }
                    }
                }
                
                if should_finish {
                    self.is_processing = false;
                    self.progress_rx = None;
                    self.start_time = None;
                    self.estimated_time = None;
                }
                
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.is_processing {
            iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Title row with icon
        let icon = iced::widget::image::Image::new("static/icon.png")
            .height(32);
        
        let title_row = row![
            icon,
            text("rs-licer").size(32),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let input_row = row![
            text("Input STL:").width(Length::Fixed(120.0)),
            text_input("Select an STL file...", &self.input_path)
                .on_input(Message::InputPathChanged)
                .width(Length::Fill),
            button("Browse").on_press(Message::BrowseFile),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let output_row = row![
            text("Output Directory:").width(Length::Fixed(120.0)),
            text_input("Output directory", &self.output_dir)
                .on_input(Message::OutputDirChanged)
                .width(Length::Fill),
            button("Browse").on_press(Message::BrowseOutputDir),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let pixel_row = row![
            text("Pixel Size (μm):").width(Length::Fixed(120.0)),
            text_input("33.3333", &self.pixel_size)
                .on_input(Message::PixelSizeChanged)
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let layer_row = row![
            text("Layer Height (μm):").width(Length::Fixed(120.0)),
            text_input("20.0", &self.layer_height)
                .on_input(Message::LayerHeightChanged)
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let checkboxes = column![
            checkbox("Zero Slice Position", self.zero_slice_position)
                .on_toggle(Message::ZeroSliceToggled),
            checkbox("Delete Below Zero", self.delete_below_zero)
                .on_toggle(Message::DeleteBelowZeroToggled),
            checkbox("Delete Output Directory", self.delete_output_dir)
                .on_toggle(Message::DeleteOutputDirToggled),
            checkbox("Open Output Directory When Done", self.open_output_dir)
                .on_toggle(Message::OpenOutputDirToggled),
        ]
        .spacing(8);

        let mut content = column![
            title_row,
            input_row,
            output_row,
            pixel_row,
            layer_row,
            checkboxes,
        ]
        .spacing(15)
        .padding(20);

        if self.is_processing {
            content = content.push(progress_bar(0.0..=1.0, self.progress));
            
            if let Some(ref time) = self.estimated_time {
                content = content.push(text(format!("Estimated time remaining: {}", time)));
            }
        }

        let slice_button = if self.is_processing {
            button("Processing...").style(button::secondary)
        } else {
            button("Slice").style(button::primary).on_press(Message::Slice)
        };

        content = content.push(slice_button);
        content = content.push(text(&self.status_message).size(14));

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::default()
    }
}
