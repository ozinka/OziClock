using System;
using System.Collections.Generic;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;

namespace Ozi.Utilities;

public partial class FmSlider : Window
{
    public DateTime CurTime;

    public FmSlider(FmMainWindow fmMain)
    {
        InitializeComponent();
        Left = fmMain.Left;
        Top = fmMain.Top + fmMain.Height;

        Loaded += Window_Loaded;
    }

    private void Window_Loaded(object sender, RoutedEventArgs e)
    {
        InitGui(App.Settings.LstClock.Count);
    }

    private void InitGui(int count)
    {
        this.Width = count * 100 + 1;
        double currentLeft = 5; // Start position
        double marginTop = 39;
        List<string> lst = new List<string>();

        // Define the list of labels based on the count
        switch (count)
        {
            case 1:
                slTimeChecker.TickFrequency = 144;
                lst = new List<string> { "0", "12", "24" };
                break;
            case 2:
                slTimeChecker.TickFrequency = 72;
                lst = new List<string> { "0", "6", "12", "18", "24" };
                break;
            case 3:
                slTimeChecker.TickFrequency = 36;
                lst = new List<string> { "0", "3", "6", "9", "12", "15", "18", "21", "24" };
                break;
            case 4:
                slTimeChecker.TickFrequency = 24;
                lst = new List<string> { "0", "2", "4", "6", "8", "10", "12", "14", "16", "18", "20", "22", "24" };
                break;
            default:
                slTimeChecker.TickFrequency = 24;
                lst = new List<string> { "0", "2", "4", "6", "8", "10", "12", "14", "16", "18", "20", "22", "24" };
                break;
        }

        // Get the DPI scale (125% or 1.25 scaling factor)
        var dpiScale = PresentationSource.FromVisual(this).CompositionTarget.TransformToDevice.M11;

        double delta = (this.Width - 27) / (lst.Count - 1); // Adjust delta to fit the available space

        // Calculate the expected width of the labels based on the font size (considering DPI scale)
        double expectedLabelWidth = 0;

        // Assuming we use the same font for all labels, you can calculate based on the font size
        foreach (string label in lst)
        {
            if (label.Length == 1)
            {
                expectedLabelWidth = 9 * dpiScale; // Single digit width
            }
            else
            {
                expectedLabelWidth = 16 * dpiScale; // Double digit width
            }

            Label lbl = new Label
            {
                Content = label,
                HorizontalAlignment = HorizontalAlignment.Left,
                VerticalAlignment = VerticalAlignment.Top,
                Foreground = new SolidColorBrush(Color.FromRgb(199, 195, 195)),
            };

            // Adjust the margin to center the label under the tick
            lbl.Margin = new Thickness(currentLeft - (expectedLabelWidth / 3), marginTop, 0, 0);

            // Add the label to the UI
            gdMain.Children.Add(lbl);

            // Move to the next tick position
            currentLeft += delta;
        }

    }
    
}