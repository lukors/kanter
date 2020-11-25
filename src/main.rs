use bevy::prelude::*;

struct Person;
struct Name(String);
pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(GreetTimer(Timer::from_seconds(2.0, true)))
            .add_startup_system(add_people.system())
            .add_system(greet_people.system());
    }
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(HelloPlugin)
        .run();
}

fn add_people(mut commands: Commands) {
    commands
        .spawn((Person, Name("Lukas".to_string())))
        .spawn((Person, Name("Max".to_string())))
        .spawn((Person, Name("Joel".to_string())));
}

struct GreetTimer(Timer);

fn greet_people(time: Res<Time>, mut timer: ResMut<GreetTimer>, query: Query<(&Person, &Name)>) {
    timer.0.tick(time.delta_seconds);

    if timer.0.finished {
        for (_person, name) in query.iter() {
            println!("hello {}!", name.0);
        }
    }
}

