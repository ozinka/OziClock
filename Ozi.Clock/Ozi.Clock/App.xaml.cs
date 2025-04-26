using System.Configuration;
using System.Data;
using System.IO;
using System.Text.Json;
using System.Windows;

namespace Ozi.Clock;

/// <summary>
/// Interaction logic for App.xaml
/// </summary>
public partial class App : Application
{
    public static AppSettings Settings { get; private set; } = null!;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        var settingsPath = Path.Combine(AppContext.BaseDirectory, "appsettings.json");

        if (File.Exists(settingsPath))
        {
            Settings = JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(settingsPath)) ??
                       throw new InvalidOperationException();
        }
        else
        {
            throw new FileNotFoundException("appsettings.json not found", settingsPath);
        }
    }
}