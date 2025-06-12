using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;

// Required for INotifyPropertyChanged

namespace Ozi.Utilities.Settings
{
    public class AppSettings
    {
        public double MainWndLeft { get; set; }
        public double MainWndTop { get; set; }

        public bool IsTransparent { get; set; }
        public double Opacity { get; set; }
        public bool TopMost { get; set; }
        public bool ShowInTaskBar { get; set; }

        public bool UseSnap { get; set; }

        public List<ClockSettings> ClocksSettings { get; set; } = [];

        public void Save()
        {
            var options = new JsonSerializerOptions
            {
                WriteIndented = true,
                // ReferenceHandler = ReferenceHandler.Preserve - ? 
            };

            var actualSettings = App
                .Clocks
                .Select(clock => new ClockSettings
                {
                    Label = clock.Caption,
                    TimeZone = clock.TimeZoneId,
                    Color = clock.Color,
                    IsMain = clock.IsMain
                })
                .ToList();

            ClocksSettings = actualSettings;
            var fileData = JsonSerializer.Serialize(this, options);

            var path = Path.Combine(AppContext.BaseDirectory, "appsettings.json");
            File.WriteAllText(path, fileData);
        }
    }
}