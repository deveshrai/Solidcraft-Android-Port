# Picked-face navigation

A face selected in the viewport uses its persistent creating feature for **Edit Feature**, **Find in Timeline** and **Find in Browser**. Later holes, blends and trims do not redirect the action to the feature that originally created the whole body.

The Browser contains a **Features** folder for each native component with non-sketch history. Sketches appear only in **Sketches**, and a sketch-only component has no Features folder. Find in Browser opens the ancestors and the Features folder in the selected body's actual occurrence, selects the creating feature and scrolls its row into view. Double-clicking the row opens its editor. This leaves the timeline layout, active component and history marker unchanged until an edit is requested.

Selecting a whole body keeps the existing body navigation. A face with an unavailable creator does not fall back to the body's base feature. Imported source identifiers do not acquire an unrelated editable native history.
