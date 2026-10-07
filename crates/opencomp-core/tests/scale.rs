use opencomp_core::action::{Action, MouseButton, Point};
use opencomp_core::scale::{scale_action, to_screen};

#[test]
fn bottom_right_of_a_half_size_image_lands_on_the_bottom_right_pixel() {
    let point = to_screen(Point { x: 49, y: 24 }, 50, 25, 100, 50);
    assert_eq!(point, Point { x: 99, y: 49 });
}

#[test]
fn equal_sizes_leave_the_point_unchanged() {
    let point = to_screen(Point { x: 1, y: 1 }, 1, 1, 1, 1);
    assert_eq!(point, Point { x: 1, y: 1 });
}

#[test]
fn type_action_is_unchanged() {
    let action = Action::Type("hello".to_owned());
    assert_eq!(scale_action(&action, 50, 25, 100, 50), action);
}

#[test]
fn click_points_are_scaled() {
    let action = Action::Click {
        point: Point { x: 10, y: 5 },
        button: MouseButton::Left,
    };
    assert_eq!(
        scale_action(&action, 50, 25, 100, 50),
        Action::Click {
            point: Point { x: 20, y: 10 },
            button: MouseButton::Left,
        }
    );
}
